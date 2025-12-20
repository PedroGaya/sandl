use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub fn impl_args(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Args can only be derived for structs with named fields"),
        },
        _ => panic!("Args can only be derived for structs"),
    };

    let from_value_fields = fields.iter().map(|f| {
        let field_name = &f.ident;
        let field_name_str = field_name.as_ref().unwrap().to_string();
        let field_type = &f.ty;

        quote! {
                let #field_name = obj.get(#field_name_str)
                    .ok_or_else(|| sandl::Error::ConfigError(
                        format!("Missing required argument '{}' in {}", #field_name_str, stringify!(#name))
                    ))?;
                let #field_name = <#field_type as sandl::FromValue>::from_value(#field_name)?;
        }
    });

    let field_names = fields.iter().map(|f| f.ident.clone()).collect::<Vec<_>>();

    let expanded = quote! {
        impl sandl::FromValue for #name {
            fn from_value(value: &sandl::Value) -> sandl::Result<Self> {
                let obj = value.as_object()
                    .ok_or_else(|| sandl::Error::ConfigError(
                        format!("Expected object for {}", stringify!(#name))
                    ))?;

                #(#from_value_fields)*

                Ok(#name {
                    #(#field_names),*
                })
            }
        }

        impl sandl::ToValue for #name {
            fn to_value(&self) -> sandl::Value {
                let mut map = std::collections::HashMap::new();
                #(
                    map.insert(
                        stringify!(#field_names).to_string(),
                        <_ as sandl::ToValue>::to_value(&self.#field_names)
                    );
                )*
                sandl::Value::Object(map)
            }
        }
    };

    TokenStream::from(expanded)
}

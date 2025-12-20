use proc_macro::TokenStream;

mod args;

#[proc_macro_derive(Args)]
pub fn derive_args(input: TokenStream) -> TokenStream {
    args::impl_args(input)
}

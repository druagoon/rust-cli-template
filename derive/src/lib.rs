use proc_macro::TokenStream;

mod command;

#[proc_macro_derive(CliCommand)]
pub fn derive_cli_command(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    command::derive_cli_command(&input).unwrap_or_else(syn::Error::into_compile_error).into()
}

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DeriveInput, Ident};

pub fn derive_cli_command(input: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let ident = &input.ident;
    match &input.data {
        Data::Enum(data) => Ok(impl_cli_command(ident, data)),
        _ => Err(syn::Error::new_spanned(input, "`CliCommand` only supports enums")),
    }
}

fn impl_cli_command(ident: &Ident, data: &DataEnum) -> TokenStream {
    let arms = data.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        let cfg_attributes = variant.attrs.iter().filter(|attribute| {
            attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
        });

        quote! {
            #(#cfg_attributes)*
            #ident::#variant_ident(command) => command.run()
        }
    });

    quote! {
        impl CliCommand for #ident {
            fn run(&self) -> CliResult {
                match self {
                    #(#arms),*
                }
            }
        }
    }
}

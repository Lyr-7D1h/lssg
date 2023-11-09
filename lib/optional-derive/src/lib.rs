use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(Optional)]
pub fn optional_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_optional_macro(&ast)
}

fn impl_optional_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        struct OptionalExternalpage {
            input: Option<PathBuf>
        }
        impl Optional<OptionalExternalpage> for #name {
            fn from_optional(&mut self, t: OptionalExternalpage) {

            }
        }
    };
    gen.into()
}

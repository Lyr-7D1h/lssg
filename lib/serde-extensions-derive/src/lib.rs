use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, Fields};

#[proc_macro_derive(Overwrite)]
pub fn optional_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_optional_macro(&ast)
}

fn impl_optional_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let optional_name = format_ident!("Optional{}", name);

    let fields = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    };

    let field_names: Vec<_> = fields.iter().map(|field| &field.ident).collect();

    let overwrite_code: Vec<_> = fields
        .iter()
        .map(|field| {
            let field_name = &field.ident;
            quote! {
                if let Some(value) = optional.#field_name {
                    // Helper trait to try overwriting if Overwrite is implemented
                    trait MaybeOverwrite {
                        fn maybe_overwrite<E: serde::de::Error>(&mut self, value: ::serde_extensions::serde_value::Value) -> Result<(), E>;
                    }
                    
                    // Implementation for types that implement Overwrite
                    impl<T: Overwrite> MaybeOverwrite for T {
                        fn maybe_overwrite<E: serde::de::Error>(&mut self, value: ::serde_extensions::serde_value::Value) -> Result<(), E> {
                            let result: Result<(), ::serde_extensions::serde_value::DeserializerError> = 
                                self.overwrite(::serde_extensions::serde_value::ValueDeserializer::new(value));
                            result.map_err(|e| E::custom(e.to_string()))?;
                            Ok(())
                        }
                    }
                    
                    self.#field_name.maybe_overwrite::<D::Error>(value)?;
                }
            }
        })
        .collect();

    let gen = quote! {
        // make an optional version of this struct that stores raw serde values
        #[derive(serde::Deserialize, Default)]
        #[serde(default)]
        struct #optional_name {
            #( #field_names: Option<::serde_extensions::serde_value::Value>, )*
        }

        // parse `d` to optional struct and check for each field if it has a value
        // if yes then overwrite `$field_name` of `#name`
        impl Overwrite for #name {
            /// Overwrite self with a serde object
            fn overwrite<'de, D>(&mut self, d: D) -> Result<(), D::Error>
            where
                D: serde::Deserializer<'de>
            {
                let optional: #optional_name = serde::de::Deserialize::deserialize(d)?;
                #( #overwrite_code )*
                return Ok(())
            }
        }
    };
    gen.into()
}

// fn is_vec_type(ty: &Type) -> bool {
//     if let Type::Path(path) = ty {
//         if let Some(segment) = path.path.segments.last() {
//             return segment.ident.to_string() == "Vec";
//         }
//     }
//     false
// }
//
// fn is_hashmap_type(ty: &Type) -> bool {
//     if let Type::Path(path) = ty {
//         if let Some(segment) = path.path.segments.last() {
//             return segment.ident.to_string() == "HashMap";
//         }
//     }
//     false
// }

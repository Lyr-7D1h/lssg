use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, Fields, Type};

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
    let field_types: Vec<_> = fields.iter().map(|field| &field.ty).collect();

    let overwrite_code: Vec<_> = fields
        .iter()
        .map(|field| {
            let field_name = &field.ident;
            // let field_type = &field.ty;

            // Generate different code based on field type
            // if is_vec_type(field_type) {
            //     quote! {
            //         if let Some(mut field) = optional.#field_name {
            //             self.#field_name.append(&mut field);
            //         }
            //     }
            // } else if is_hashmap_type(field_type) {
            //     quote! {
            //         if let Some(mut field) = optional.#field_name {
            //             self.#field_name.extend(field);
            //         }
            //     }
            // } else {
            // }
            quote! {
                if let Some(field) = optional.#field_name {
                    self.#field_name = field;
                }
            }
        })
        .collect();

    let gen = quote! {
        #[derive(serde::Deserialize)]
        struct #optional_name {
            #( #field_names: Option<#field_types>, )*
        }

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

fn is_vec_type(ty: &Type) -> bool {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            return segment.ident.to_string() == "Vec";
        }
    }
    false
}

fn is_hashmap_type(ty: &Type) -> bool {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            return segment.ident.to_string() == "HashMap";
        }
    }
    false
}

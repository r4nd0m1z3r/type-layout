extern crate proc_macro;

use proc_macro::TokenStream;

use proc_macro2::{Ident, Literal};
use quote::{quote, quote_spanned};
use syn::{Data, DeriveInput, Fields, parse_macro_input, spanned::Spanned, TypeGenerics};

#[proc_macro_derive(TypeLayout)]
pub fn derive_type_layout(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let name = input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let layout = layout_of_type(&name, &ty_generics, &input.data);

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
        impl #impl_generics ::type_layout::TypeLayout for #name #ty_generics #where_clause {
            fn type_layout() -> ::type_layout::TypeLayoutInfo {
                let mut last_field_end = 0;
                let mut fields = Vec::new();

                #layout

                fields.sort_by_key(|f| f.offset);

                ::type_layout::TypeLayoutInfo {
                    name: ::std::borrow::Cow::Borrowed(::std::any::type_name::<Self>()),
                    size: std::mem::size_of::<#name #ty_generics>(),
                    alignment: ::std::mem::align_of::<#name #ty_generics>(),
                    fields,
                }
            }
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}

fn layout_of_type(struct_name: &Ident, ty_generics: &TypeGenerics, data: &Data) -> proc_macro2::TokenStream {
    match data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let values = fields.named.iter().map(|field| {
                    let field_name = field.ident.as_ref().unwrap();
                    let field_name_str = Literal::string(&field_name.to_string());
                    let field_ty = &field.ty;

                    quote_spanned! { field.span() =>
                        #[allow(unused_assignments)]
                        {
                            let size = ::std::mem::size_of::<#field_ty>();
                            let offset = ::type_layout::memoffset::offset_of!(#struct_name #ty_generics, #field_name);

                            fields.push(::type_layout::Field {
                                name: ::std::borrow::Cow::Borrowed(#field_name_str),
                                ty: ::std::borrow::Cow::Borrowed(::std::any::type_name::<#field_ty>()),
                                size,
                                offset,
                            });

                            last_field_end = offset + size;
                        }
                    }
                });

                quote! {
                    #(#values)*
                }
            }
            Fields::Unnamed(_) => unimplemented!(),
            Fields::Unit => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!("type-layout only supports structs"),
    }
}

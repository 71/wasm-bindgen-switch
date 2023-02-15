use proc_macro2::TokenStream;
use proc_macro_error::{emit_error, proc_macro_error, ResultExt};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;

/// When targeting `wasm`, replaces the item as specified by the
/// `wasm-bindgen-switch` package documentation. When targeting another family,
/// keeps the item as-is.
#[proc_macro_error]
#[proc_macro_attribute]
pub fn wasm_bindgen_switch(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let options: SwitchOptions = syn::parse2(attr.into()).unwrap_or_abort();
    let mut item: syn::Item = syn::parse2(input.into()).unwrap_or_abort();
    let extern_contents = item_as_wasm_bindgen_imports(&mut item, &options, &quote! {});

    remove_wasm_bindgen_attrs(&mut item);

    let added_attrs = options.other_args;

    quote! {
        #[cfg(target_family = "wasm")]
        #[wasm_bindgen::prelude::wasm_bindgen(#( #added_attrs ),*)]
        extern "C" {
            #extern_contents
        }

        #[cfg(not(target_family = "wasm"))]
        #item
    }
    .into()
}

/// Attribute macro replaced by `#[wasm_bindgen_test]` when targeting a `wasm`
/// target, and `#[test]` otherwise.
#[proc_macro_error]
#[proc_macro_attribute]
pub fn wasm_bindgen_switch_test(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    ensure_attr_is_empty(attr);

    let input = TokenStream::from(input);

    quote! {
        #[cfg(target_family = "wasm")]
        #[wasm_bindgen_test::wasm_bindgen_test]
        #input

        #[cfg(not(target_family = "wasm"))]
        #[test]
        #input
    }
    .into()
}

/// Aborts the macro invocation if the given token stream is non-empty.
fn ensure_attr_is_empty(attr: proc_macro::TokenStream) {
    if !attr.is_empty() {
        let attr = TokenStream::from(attr);

        emit_error!(attr, "arguments are not supported");
    }
}

/// Converts an item to a `wasm_bindgen` JS import.
///
/// `item` is taken mutably to replace some types, avoiding the need to clone
/// it. This does not impact the rest of the transformation.
fn item_as_wasm_bindgen_imports(
    item: &mut syn::Item,
    options: &SwitchOptions,
    namespace_attr: &TokenStream,
) -> TokenStream {
    // See https://rustwasm.github.io/docs/wasm-bindgen/reference/attributes/on-js-imports/index.html.
    match item {
        syn::Item::Enum(syn::ItemEnum {
            attrs, ident, vis, ..
        })
        | syn::Item::Struct(syn::ItemStruct {
            attrs, ident, vis, ..
        })
        | syn::Item::Type(syn::ItemType {
            attrs, ident, vis, ..
        })
        | syn::Item::Union(syn::ItemUnion {
            attrs, ident, vis, ..
        }) => {
            // Replace data types with `type` imports.
            quote! {
                #namespace_attr
                #( #attrs )*
                #vis type #ident;
            }
        }

        syn::Item::Mod(syn::ItemMod {
            attrs,
            content: Some((_, items)),
            ident,
            mod_token,
            semi: _,
            vis,
        }) => {
            // Replace all items in the mod recursively.
            let namespace = syn::LitStr::new(&ident.to_string(), ident.span());
            let namespace_attr = quote! { #namespace };
            let items = items
                .iter_mut()
                .map(move |item| item_as_wasm_bindgen_imports(item, options, &namespace_attr));

            quote! {
                #( #attrs )*
                #vis #mod_token #ident {
                    #( #items )*
                }
            }
        }

        syn::Item::Fn(syn::ItemFn {
            attrs,
            block: _,
            sig,
            vis,
        }) => {
            // Replace function with import.
            let js_name_attr = options.make_js_name_attr(&sig.ident);

            quote! {
                #[wasm_bindgen]
                #namespace_attr
                #( #attrs )*
                #js_name_attr
                #vis #sig;
            }
        }

        syn::Item::Impl(syn::ItemImpl {
            attrs,
            items,
            self_ty,
            trait_: None,
            ..
        }) => {
            // Replace with function imports.
            let syn::Type::Path(syn::TypePath { path: self_path, qself: None }) = &**self_ty else {
                emit_error!(self_ty, "unsupported impl type");

                return TokenStream::new();
            };
            let Some(self_ident) = self_path.get_ident() else {
                emit_error!(self_path, "impl type must be a single identifier");

                return TokenStream::new();
            };

            let items = items.iter_mut().map(|item| {
                let syn::ImplItem::Method(method) = item else {
                    emit_error!(item, "unsupported impl item");

                    return TokenStream::new();
                };
                let method_tokens = method_as_wasm_bindgen_import(method, self_ident, options);

                quote! {
                    #( #attrs )*
                    #method_tokens
                }
            });

            quote! {
                #( #items )*
            }
        }

        _ => {
            emit_error!(item, "unsupported item");

            TokenStream::new()
        }
    }
}

/// Converts a method to a `wasm_bindgen` JS import.
fn method_as_wasm_bindgen_import(
    method: &mut syn::ImplItemMethod,
    self_ty: &syn::Ident,
    options: &SwitchOptions,
) -> TokenStream {
    let syn::ImplItemMethod {
        attrs, sig, vis, ..
    } = method;
    let syn::Signature {
        fn_token,
        ident,
        inputs,
        output,
        ..
    } = sig;
    let js_name_attr = options.make_js_name_attr(ident);

    let mut inputs = inputs.iter_mut();
    let (attr_tokens, first_input_tokens) = match inputs.next() {
        Some(syn::FnArg::Receiver(syn::Receiver {
            mutability,
            reference,
            ..
        })) => {
            // Instance method.
            let ref_mut = match (reference, mutability) {
                (Some((ref_, lt)), Some(mut_)) => quote! { _: #ref_ #lt #mut_ #self_ty },
                (Some((ref_, lt)), None) => quote! { _: #ref_ #lt #self_ty },
                (None, None | Some(_)) => quote! { _: #self_ty },
            };

            (
                quote! {
                    #[wasm_bindgen(method)]
                },
                ref_mut,
            )
        }
        Some(syn::FnArg::Typed(typed)) => {
            // Static method.
            replace_self_with_ty(&mut typed.ty, self_ty);

            let attr = if attrs.iter().any(|x| x.is_wasm_bindgen_constructor_attr()) {
                // If the user explicitly requests a constructor, don't turn it
                // into a static method.
                TokenStream::new()
            } else {
                quote! {
                    #[wasm_bindgen(static_method_of = #self_ty)]
                }
            };

            (attr, quote! { #typed })
        }
        None => (
            quote! {
                #[wasm_bindgen(static_method_of = #self_ty)]
            },
            TokenStream::new(),
        ),
    };
    let inputs = inputs.map(|input| {
        if let syn::FnArg::Typed(pat) = input {
            replace_self_with_ty(&mut pat.ty, self_ty);
        } else {
            emit_error!(input, "invalid receiver");
        }

        input
    });

    if let syn::ReturnType::Type(_, ty) = output {
        replace_self_with_ty(ty, self_ty);
    }

    quote! {
        #( #attrs )*
        #attr_tokens
        #js_name_attr
        #vis #fn_token #ident( #first_input_tokens #( , #inputs )* ) #output;
    }
}

/// Rewrites the given type so that `Self` is replaced to a reference to the
/// given type.
fn replace_self_with_ty(ty: &mut syn::Type, self_ty: &syn::Ident) {
    struct ReplaceSelfWithTy<'a>(&'a syn::Ident);

    impl syn::visit_mut::VisitMut for ReplaceSelfWithTy<'_> {
        fn visit_type_path_mut(&mut self, i: &mut syn::TypePath) {
            if i.path.is_ident("Self") {
                i.path.segments[0].ident = self.0.clone();
            }
        }
    }

    syn::visit_mut::visit_type_mut(&mut ReplaceSelfWithTy(self_ty), ty);
}

/// Rewrites the given item, removing all `#[wasm_bindgen]` attributes.
fn remove_wasm_bindgen_attrs(item: &mut syn::Item) {
    struct RemoveWasmBindgenAttrs;

    fn remove_wasm_bindgen_attrs(attrs: &mut Vec<syn::Attribute>) {
        attrs.retain(|attr| !attr.is_wasm_bindgen_attr());
    }

    macro_rules! impl_visit_mut {
        ( $($visit_fn_name: ident $(+ $($_: ty)?)?: $ty: ty,)* ) => {
            impl syn::visit_mut::VisitMut for RemoveWasmBindgenAttrs {
                $(fn $visit_fn_name(&mut self, i: &mut $ty) {
                    remove_wasm_bindgen_attrs(&mut i.attrs);

                    $(
                        $($_)?  // Needed to identify that `+` was given.

                        syn::visit_mut::$visit_fn_name(self, i);
                    )?
                })*
            }
        };
    }

    // Note: we only visit a subset of all nodes that may have attributes, as
    // `#[wasm_bindgen]` can only be attached to these nodes. Names with a `+`
    // are visited recursively.
    impl_visit_mut!(
        visit_item_const_mut+: syn::ItemConst,
        visit_item_fn_mut+: syn::ItemFn,
        visit_item_impl_mut+: syn::ItemImpl,

        // We also visit these nodes, as they are valid within a `switch` item.
        visit_impl_item_method_mut+: syn::ImplItemMethod,
        visit_item_enum_mut: syn::ItemEnum,
        visit_item_struct_mut: syn::ItemStruct,
        visit_item_type_mut: syn::ItemType,
        visit_item_union_mut: syn::ItemUnion,
    );

    syn::visit_mut::visit_item_mut(&mut RemoveWasmBindgenAttrs, item);
}

#[derive(Default)]
struct SwitchOptions {
    camel_case: bool,
    other_args: Vec<TokenStream>,
}

impl syn::parse::Parse for SwitchOptions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut result = SwitchOptions::default();
        let meta = input.parse_terminated::<_, syn::Token![,]>(syn::Meta::parse)?;

        for meta in meta {
            match meta {
                syn::Meta::Path(path) if path.is_ident("camel_case") => result.camel_case = true,
                other => result.other_args.push(other.into_token_stream()),
            }
        }

        Ok(result)
    }
}

impl SwitchOptions {
    /// Returns a token stream like `#[wasm_bindgen(js_name = "...")]` if
    /// [`Self::camel_case`] is true, else returns an empty token stream.
    fn make_js_name_attr(&self, name: &syn::Ident) -> TokenStream {
        if !self.camel_case {
            return TokenStream::new();
        }

        let snake_case_name = name.to_string();
        let underscores = snake_case_name.chars().filter(|x| *x == '_').count();

        if underscores == 0 {
            return TokenStream::new();
        }

        let mut camel_case_name = String::with_capacity(snake_case_name.len() - underscores);
        let mut had_underscore = false;

        for ch in snake_case_name.chars() {
            if had_underscore {
                if ch == '_' {
                    emit_error!(name, "double underscore in modified identifier");
                }

                camel_case_name.extend(ch.to_uppercase());
                had_underscore = false;

                continue;
            }

            if ch == '_' {
                had_underscore = true;

                continue;
            }

            camel_case_name.push(ch);
        }

        let camel_case_name_lit = syn::LitStr::new(&camel_case_name, name.span());

        quote! {
            #[wasm_bindgen(js_name = #camel_case_name_lit)]
        }
    }
}

trait AttributeExt {
    fn is_wasm_bindgen_attr(&self) -> bool;
    fn is_wasm_bindgen_constructor_attr(&self) -> bool;
}

impl AttributeExt for syn::Attribute {
    fn is_wasm_bindgen_attr(&self) -> bool {
        self.path
            .segments
            .first()
            .map(|x| x.arguments.is_empty() && x.ident == "wasm_bindgen")
            .unwrap_or(false)
    }

    fn is_wasm_bindgen_constructor_attr(&self) -> bool {
        if !self.is_wasm_bindgen_attr() {
            return false;
        }

        let Ok(meta) = self.parse_meta() else { return false };

        match meta {
            syn::Meta::Path(path) => path.is_ident("constructor"),
            syn::Meta::List(list) => list.nested.iter().any(|x| {
                matches!(
                            x,
                            syn::NestedMeta::Meta(syn::Meta::Path(path))
                                if path.is_ident("constructor"))
            }),
            syn::Meta::NameValue(_) => false,
        }
    }
}

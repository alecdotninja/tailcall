use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, ImplItemMethod, ItemFn, Signature};

use crate::{
    analyze::{is_simple_self_tail_recursive, is_simple_self_tail_recursive_method},
    loop_lower::{lower_self_tail_loop, lower_self_tail_method_loop},
    rewrite::TailPositionRewriter,
    signature::{function_argument_exprs, helper_signature, method_helper_signature},
};

pub fn apply_fn_tailcall_transform(item_fn: ItemFn) -> TokenStream {
    match TailcallTransform::new(item_fn).expand() {
        Ok(output) => output,
        Err(error) => error.to_compile_error(),
    }
}

pub fn apply_method_tailcall_transform(method: ImplItemMethod) -> TokenStream {
    match TailcallMethodTransform::new(method).expand() {
        Ok(output) => output,
        Err(error) => error.to_compile_error(),
    }
}

struct TailcallTransform {
    item_fn: ItemFn,
}

struct TailcallMethodTransform {
    method: ImplItemMethod,
}

impl TailcallMethodTransform {
    fn new(method: ImplItemMethod) -> Self {
        Self { method }
    }

    fn expand(self) -> Result<TokenStream, Error> {
        let ImplItemMethod {
            attrs,
            vis,
            defaultness,
            sig,
            block,
        } = self.method;

        reject_unsupported_signature(&sig)?;

        let helper_sig = method_helper_signature(&sig)?;
        let helper_fn_ident = &helper_sig.ident;
        let helper_args = function_argument_exprs(&sig)?;
        let original_method = ImplItemMethod {
            attrs: attrs.clone(),
            vis: vis.clone(),
            defaultness,
            sig: sig.clone(),
            block: block.clone(),
        };
        let optimized = is_simple_self_tail_recursive_method(&original_method);
        let wrapper_body = if optimized {
            lower_self_tail_method_loop(&original_method)?
        } else {
            quote! { Self::#helper_fn_ident(#(#helper_args),*).call() }
        };
        let helper_body = if optimized {
            let method_ident = &sig.ident;
            quote! {
                tailcall::runtime::Thunk::value(Self::#method_ident(#(#helper_args),*))
            }
        } else {
            let helper_block = TailPositionRewriter::rewrite(block)?;
            quote! {
                tailcall::runtime::Thunk::bounce(move || #helper_block)
            }
        };

        Ok(quote! {
            #(#attrs)*
            #defaultness #vis #sig {
                #wrapper_body
            }

            #[doc(hidden)]
            #[allow(unused)]
            #[inline(always)]
            #helper_sig {
                #helper_body
            }
        })
    }
}

impl TailcallTransform {
    fn new(item_fn: ItemFn) -> Self {
        Self { item_fn }
    }

    fn expand(self) -> Result<TokenStream, Error> {
        let ItemFn {
            attrs,
            vis,
            sig,
            block,
        } = self.item_fn;

        reject_unsupported_signature(&sig)?;

        let helper_sig = helper_signature(&sig);
        let helper_fn_ident = &helper_sig.ident;
        let helper_args = function_argument_exprs(&sig)?;
        let original_item_fn = ItemFn {
            attrs: attrs.clone(),
            vis: vis.clone(),
            sig: sig.clone(),
            block: block.clone(),
        };
        let optimized = is_simple_self_tail_recursive(&original_item_fn);
        let wrapper_body = if optimized {
            lower_self_tail_loop(&original_item_fn)?
        } else {
            quote! { #helper_fn_ident(#(#helper_args),*).call() }
        };
        let helper_body = if optimized {
            let fn_ident = &sig.ident;
            quote! {
                tailcall::runtime::Thunk::value(#fn_ident(#(#helper_args),*))
            }
        } else {
            let helper_block = TailPositionRewriter::rewrite(*block)?;
            quote! {
                tailcall::runtime::Thunk::bounce(move || #helper_block)
            }
        };

        Ok(quote! {
            #(#attrs)*
            #vis #sig {
                #wrapper_body
            }

            #[doc(hidden)]
            #[allow(unused)]
            #[inline(always)]
            #helper_sig {
                #helper_body
            }
        })
    }
}

fn reject_unsupported_signature(sig: &Signature) -> Result<(), Error> {
    if sig.constness.is_some() {
        return Err(Error::new_spanned(
            sig.constness,
            "#[tailcall] does not support const functions",
        ));
    }

    if sig.asyncness.is_some() {
        return Err(Error::new_spanned(
            sig.asyncness,
            "#[tailcall] does not support async functions",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::parse_quote;

    use super::{apply_fn_tailcall_transform, apply_method_tailcall_transform};

    fn assert_expansion_eq(actual: TokenStream, expected: TokenStream) {
        assert_eq!(actual.to_string(), expected.to_string());
    }

    #[test]
    fn expands_runtime_backed_free_function_as_expected() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn is_even(x: u32) -> bool {
                if x == 0 {
                    true
                } else {
                    tailcall::call! { is_odd(x - 1) }
                }
            }
        };

        let actual = apply_fn_tailcall_transform(item_fn);
        let expected = quote! {
            fn is_even(x: u32) -> bool {
                __tailcall_build_is_even_thunk(x).call()
            }

            #[doc(hidden)]
            #[allow(unused)]
            #[inline(always)]
            fn __tailcall_build_is_even_thunk<'tailcall>(x: u32) -> tailcall::runtime::Thunk<'tailcall, bool> {
                tailcall::runtime::Thunk::bounce(move || {
                    if x == 0 {
                        tailcall::runtime::Thunk::value(true)
                    } else {
                        tailcall::call! { is_odd(x - 1) }
                    }
                })
            }
        };

        assert_expansion_eq(actual, expected);
    }

    #[test]
    fn expands_loop_lowered_free_function_as_expected() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn countdown(n: u32) -> u32 {
                if n > 0 {
                    tailcall::call! { countdown(n - 1) }
                } else {
                    0
                }
            }
        };

        let actual = apply_fn_tailcall_transform(item_fn);
        let expected = quote! {
            fn countdown(n: u32) -> u32 {
                let mut n = n;
                loop {
                    if n > 0 {
                        {
                            let __tailcall_next_0 = n - 1;
                            n = __tailcall_next_0;
                            continue;
                        }
                    } else {
                        return 0
                    }
                }
            }

            #[doc(hidden)]
            #[allow(unused)]
            #[inline(always)]
            fn __tailcall_build_countdown_thunk<'tailcall>(n: u32) -> tailcall::runtime::Thunk<'tailcall, u32> {
                tailcall::runtime::Thunk::value(countdown(n))
            }
        };

        assert_expansion_eq(actual, expected);
    }

    #[test]
    fn expands_loop_lowered_method_as_expected() {
        let method: syn::ImplItemMethod = parse_quote! {
            fn countdown(&mut self, n: u32) -> u32 {
                self.steps += 1;

                if n > 0 {
                    tailcall::call! { self.countdown(n - 1) }
                } else {
                    self.steps as u32
                }
            }
        };

        let actual = apply_method_tailcall_transform(method);
        let expected = quote! {
            fn countdown(&mut self, n: u32) -> u32 {
                let __tailcall_self = self;
                let mut n = n;
                loop {
                    __tailcall_self.steps += 1;
                    if n > 0 {
                        {
                            let __tailcall_next_0 = n - 1;
                            n = __tailcall_next_0;
                            continue;
                        }
                    } else {
                        return __tailcall_self.steps as u32
                    }
                }
            }

            #[doc(hidden)]
            #[allow(unused)]
            #[inline(always)]
            fn __tailcall_build_countdown_thunk<'tailcall>(&'tailcall mut self, n: u32) -> tailcall::runtime::Thunk<'tailcall, u32> {
                tailcall::runtime::Thunk::value(Self::countdown(self, n))
            }
        };

        assert_expansion_eq(actual, expected);
    }
}

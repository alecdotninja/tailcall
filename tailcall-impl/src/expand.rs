use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, ImplItemMethod, ItemFn, Signature};

use crate::{
    analyze::is_simple_self_tail_recursive,
    loop_lower::lower_self_tail_loop,
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
        let helper_block = TailPositionRewriter::rewrite(block)?;

        Ok(quote! {
            #(#attrs)*
            #defaultness #vis #sig {
                Self::#helper_fn_ident(#(#helper_args),*).call()
            }

            #[doc(hidden)]
            #[allow(unused)]
            #[inline(always)]
            #helper_sig {
                tailcall::runtime::Thunk::bounce(move || #helper_block)
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

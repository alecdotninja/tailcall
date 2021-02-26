use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{FnArg, Ident, Pat, PatIdent, PatType, Signature};

pub trait SignatureExt {
    fn input_pat_idents(&self) -> Vec<&PatIdent>;

    fn input_pat_idents_outer(&self) -> Punctuated<FnArg, Comma>;

    fn input_idents(&self) -> Vec<Ident> {
        self.input_pat_idents()
            .iter()
            .map(|PatIdent { ident, .. }| ident.clone())
            .collect()
    }
}

impl SignatureExt for Signature {
    fn input_pat_idents(&self) -> Vec<&PatIdent> {
        self.inputs
            .iter()
            .filter_map(|fn_arg| {
                match fn_arg {
                    FnArg::Typed(PatType { pat, .. }) => {
                        if let Pat::Ident(ref pat_ident) = **pat {
                            Some(pat_ident)
                        } else {
                            unimplemented!("tail recursion with non-trivial patterns in argument list")
                        }
                    },
                    FnArg::Receiver(_) => {
                        unimplemented!("tail recursion in methods (functions with `self` in the arguments list) is not supported")
                    },
                }
            })
            .collect()
    }

    fn input_pat_idents_outer(&self) -> Punctuated<FnArg, Comma> {
        self.inputs
            .iter()
            .map(|fn_arg| {
                match fn_arg {
                    FnArg::Typed(PatType { attrs, pat, colon_token, ty }) => {
                        if let Pat::Ident(ref pat_ident) = **pat {
                            match pat_ident.mutability {
                                Some(_) => {
                                    FnArg::Typed(PatType{
                                        attrs: attrs.clone(),
                                        pat: Box::new(Pat::Ident(PatIdent {
                                            mutability: None,
                                            attrs: pat_ident.attrs.clone(),
                                            ident: pat_ident.ident.clone(),
                                            subpat: pat_ident.subpat.clone(),
                                            ..*pat_ident
                                        })),
                                        colon_token: *colon_token,
                                        ty: ty.clone()
                                    })
                                },
                                None => { fn_arg.clone() },
                            }
                        } else {
                            unimplemented!("tail recursion with non-trivial patterns in argument list")
                        }
                    },
                    FnArg::Receiver(_) => {
                        unimplemented!("tail recursion in methods (functions with `self` in the arguments list) is not supported")
                    },
                }
            })
        .collect()
    }
}

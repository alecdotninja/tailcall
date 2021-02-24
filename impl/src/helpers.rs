use syn::{FnArg, Ident, Pat, PatIdent, PatType, Signature};

pub trait SignatureExt {
    fn input_pat_idents(&self) -> Vec<&PatIdent>;

    fn input_pat_idents_outer(&self) -> Vec<PatIdent>;

    fn input_idents(&self) -> Vec<Ident> {
        self.input_pat_idents()
            .iter()
            .map(|PatIdent { ident, .. }| ident.clone())
            .collect()
    }

    fn input_idents_outer(&self) -> Vec<Ident> {
        self.input_pat_idents_outer()
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

    fn input_pat_idents_outer(&self) -> Vec<PatIdent> {
        self.inputs
            .iter()
            .filter_map(|fn_arg| {
                match fn_arg {
                    FnArg::Typed(PatType { pat, .. }) => {
                        if let Pat::Ident(ref pat_ident) = **pat {
                            match pat_ident.mutability {
                                Some(_) => {
                                // FIXME: this seems to do the opposite of what we want,
                                //        and removes `mut` from the closure.
                                    Some(PatIdent {
                                        mutability: None,
                                        attrs: pat_ident.attrs.clone(),
                                        ident: pat_ident.ident.clone(),
                                        subpat: pat_ident.subpat.clone(),
                                        ..*pat_ident
                                    })
                                },
                                None => { Some(pat_ident.clone()) },
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

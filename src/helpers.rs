use syn::{FnArg, Ident, Pat, PatIdent, PatType, Signature};

pub trait SignatureExt {
    fn input_pat_idents(&self) -> Vec<&PatIdent>;

    fn input_idents(&self) -> Vec<&Ident> {
        self.input_pat_idents()
            .iter()
            .map(|PatIdent { ident, .. }| ident)
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
}

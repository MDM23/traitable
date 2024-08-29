use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use proc_macro2::TokenTree;
use syn::{token::Impl, Item, ItemImpl, ItemMod, ItemStruct, Meta, Type};

#[derive(Debug)]
pub struct File {
    items: Vec<AstNode>,
}

#[derive(Debug)]
pub enum AstNode {
    Module { name: String, items: Vec<AstNode> },
    Struct { name: String },
    TraitImpl { trait_name: String, target: String },
    Derive { trait_name: String, target: String },
}

#[derive(Debug)]
pub struct Context {
    search_path_stack: Vec<PathBuf>,
    module_name_stack: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct Implementer {
    pub name: String,
    pub path: String,
}

impl From<&str> for Implementer {
    fn from(value: &str) -> Self {
        let mut segments: Vec<&str> = value.split("::").collect();
        let name = segments.pop().unwrap_or_default();

        Self {
            name: name.to_string(),
            path: segments.join("::"),
        }
    }
}

impl Context {
    fn push_mod(&mut self, name: &str) {
        self.module_name_stack.push(name.to_string());

        self.search_path_stack.push(
            self.search_path_stack
                .last()
                .unwrap()
                .join(&name)
                .to_path_buf(),
        );
    }

    fn pop_mod(&mut self) {
        self.search_path_stack.pop();
        self.module_name_stack.pop();
    }
}

pub fn parse(entry: impl AsRef<Path>) -> Result<File, ()> {
    let entry = entry.as_ref();

    let Some(filename) = entry.file_name().and_then(OsStr::to_str) else {
        return Err(());
    };

    if !matches!(filename, "lib.rs" | "main.rs") {
        return Err(());
    }

    let mut ctx = Context {
        search_path_stack: vec![entry.parent().unwrap().to_path_buf()],
        module_name_stack: vec![],
    };

    Ok(File {
        items: parse_module_file(&mut ctx, entry)?,
    })
}

fn parse_module_file(mut ctx: &mut Context, file: impl AsRef<Path>) -> Result<Vec<AstNode>, ()> {
    let file = file.as_ref();

    let source = std::fs::read_to_string(file).map_err(|_| ())?;

    Ok(syn::parse_file(&source).unwrap().items.to_ast(&mut ctx))
}

trait ToAst {
    fn to_ast(self, ctx: &mut Context) -> Vec<AstNode>;
}

impl ToAst for Item {
    fn to_ast(self, mut ctx: &mut Context) -> Vec<AstNode> {
        match self {
            Item::Impl(i) => i.to_ast(&mut ctx),
            Item::Mod(m) => m.to_ast(&mut ctx),
            Item::Struct(s) => s.to_ast(&mut ctx),
            _ => Vec::new(),
        }
    }
}

impl ToAst for Vec<Item> {
    fn to_ast(self, mut ctx: &mut Context) -> Vec<AstNode> {
        self.into_iter().flat_map(|i| i.to_ast(&mut ctx)).collect()
    }
}

impl ToAst for ItemMod {
    fn to_ast(self, mut ctx: &mut Context) -> Vec<AstNode> {
        let mod_name = self.ident.to_string();
        let search = ctx.search_path_stack.last().cloned().unwrap();

        ctx.push_mod(&mod_name);

        let items = match self.content {
            Some((_, items)) => items.to_ast(&mut ctx),
            None => {
                let mod_file = [
                    search.join(&mod_name).with_extension("rs"),
                    search.join(&mod_name).join("mod.rs"),
                ]
                .into_iter()
                .find(|p| p.exists());

                if let Some(f) = mod_file {
                    parse_module_file(ctx, f).unwrap()
                } else {
                    Vec::new()
                }
            }
        };

        ctx.pop_mod();

        vec![AstNode::Module {
            name: self.ident.to_string(),
            items,
        }]
    }
}

impl ToAst for ItemStruct {
    fn to_ast(self, _: &mut Context) -> Vec<AstNode> {
        let mut ast = vec![AstNode::Struct {
            name: self.ident.to_string(),
        }];

        for attr in self.attrs {
            if let Meta::List(l) = attr.meta {
                if let Some(p) = l.path.segments.first() {
                    if p.ident.to_string() == "derive" {
                        let mut tokens = l.tokens.into_iter().peekable();

                        loop {
                            let Some(t) = tokens.next() else {
                                break;
                            };

                            match (t, tokens.peek()) {
                                (TokenTree::Ident(id), Some(TokenTree::Punct(p)))
                                    if p.as_char() == ',' =>
                                {
                                    ast.push(AstNode::Derive {
                                        trait_name: id.to_string(),
                                        target: self.ident.to_string(),
                                    });
                                }
                                (TokenTree::Ident(id), None) => {
                                    ast.push(AstNode::Derive {
                                        trait_name: id.to_string(),
                                        target: self.ident.to_string(),
                                    });
                                }
                                _ => (),
                            }
                        }
                    }
                }
            }
        }

        ast
    }
}

impl ToAst for ItemImpl {
    fn to_ast(self, _: &mut Context) -> Vec<AstNode> {
        let ItemImpl {
            trait_: Some((_, tr, _)),
            self_ty: ty,
            ..
        } = self
        else {
            return Vec::new();
        };

        let Type::Path(p) = ty.as_ref() else {
            return Vec::new();
        };

        vec![AstNode::TraitImpl {
            trait_name: tr.segments.last().unwrap().ident.to_string(),
            target: p.path.segments.last().unwrap().ident.to_string(),
        }]
    }
}

impl File {
    pub fn lookup(&self, trait_subject: &str) -> Vec<Implementer> {
        fn find_impl(items: &[AstNode], path: &str, trait_subject: &str) -> Vec<Implementer> {
            let mut result = Vec::new();

            for i in items {
                match i {
                    AstNode::Module { name, items } => result.append(&mut find_impl(
                        items,
                        &(path.to_string() + "::" + name),
                        trait_subject,
                    )),
                    AstNode::TraitImpl { trait_name, target } => {
                        if trait_name == trait_subject {
                            result.push(Implementer {
                                path: path.to_string(),
                                name: target.to_string(),
                            });
                        }
                    }
                    AstNode::Derive { trait_name, target } => {
                        if trait_name == trait_subject {
                            result.push(Implementer {
                                path: path.to_string(),
                                name: target.to_string(),
                            });
                        }
                    }
                    _ => (),
                }
            }

            result
        }

        find_impl(&self.items, "crate", trait_subject)
    }
}

// #[test]
// fn test() {
//     let result = parse("examples/basic/main.rs").unwrap();
//     let implementers = result.lookup("MyTrait");

//     assert_eq!(
//         implementers,
//         [
//             "crate::module_a::module_e::ModuleEStruct",
//             "crate::module_a::ModuleAStruct",
//             "crate::module_b::module_c::ModuleCStruct",
//             "crate::module_b::ModuleBStruct",
//             "crate::MainStruct",
//         ]
//         .into_iter()
//         .map(Into::into)
//         .collect::<Vec<_>>()
//     );

//     let implementers = result.lookup("Clone");

//     assert_eq!(
//         implementers,
//         [
//             "crate::module_a::module_e::ModuleEStruct",
//             "crate::module_a::ModuleAStruct",
//         ]
//         .into_iter()
//         .map(Into::into)
//         .collect::<Vec<_>>()
//     );
// }

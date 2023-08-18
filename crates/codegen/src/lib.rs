use std::collections::HashSet;

use pg_query_proto_parser::{Node, ProtoParser, Token};
use proc_macro2::{Ident, Literal};
use quote::{format_ident, quote};

#[proc_macro]
pub fn syntax_kind(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syntax_kind_mod(item.into()).into()
}

fn syntax_kind_mod(_item: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let parser = ProtoParser::new("./crates/parser/proto/source.proto");
    let proto_file = parser.parse();

    let mut current_enum_names: HashSet<&str> = HashSet::new();

    let custom_node_names = custom_node_names();
    let custom_node_identifiers = custom_node_identifiers(&custom_node_names);
    current_enum_names.extend(&custom_node_names);

    let node_identifiers = node_identifiers(&proto_file.nodes, &current_enum_names);
    current_enum_names.extend(node_names(&proto_file.nodes));

    let token_identifiers = token_identifiers(&proto_file.tokens, &current_enum_names);
    let token_value_literals = token_value_literals(&proto_file.tokens, &current_enum_names);

    let syntax_kind_type = syntax_kind_type();
    let syntax_kind_impl =
        syntax_kind_impl(&node_identifiers, &token_identifiers, &token_value_literals);

    quote! {
        use cstree::Syntax;
        use pg_query::{protobuf::ScanToken, NodeEnum, NodeRef};

        /// An u32 enum of all valid syntax elements (nodes and tokens) of the postgres
        /// sql dialect, and a few custom ones that are not parsed by pg_query.rs, such
        /// as `Whitespace`.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Syntax)]
        #[repr(u32)]
        pub enum SyntaxKind {
            // custom nodes, which are not parsed by pg_query.rs
            #(#custom_node_identifiers),*,
            #(#node_identifiers),*,
            #(#token_identifiers),*,
        }

        #syntax_kind_type

        #syntax_kind_impl
    }
}

fn custom_node_names() -> Vec<&'static str> {
    vec![
        "SourceFile",
        "Comment",
        "Whitespace",
        "Newline",
        "Tab",
        "Stmt",
    ]
}

fn node_names(nodes: &[Node]) -> impl Iterator<Item = &str> {
    nodes.iter().map(|node| node.name.as_str())
}

fn custom_node_identifiers(custom_node_names: &[&str]) -> Vec<Ident> {
    custom_node_names
        .iter()
        .map(|&node_name| format_ident!("{}", node_name))
        .collect()
}

fn node_identifiers(nodes: &[Node], existing_enum_names: &HashSet<&str>) -> Vec<Ident> {
    nodes
        .iter()
        .filter(|&token| !existing_enum_names.contains(token.name.as_str()))
        .map(|node| format_ident!("{}", &node.name))
        .collect()
}

fn token_identifiers(tokens: &[Token], existing_enum_names: &HashSet<&str>) -> Vec<Ident> {
    tokens
        .iter()
        .filter(|&token| !existing_enum_names.contains(token.name.as_str()))
        .map(|token| format_ident!("{}", &token.name))
        .collect()
}

fn token_value_literals(tokens: &[Token], existing_enum_names: &HashSet<&str>) -> Vec<Literal> {
    tokens
        .iter()
        .filter(|&token| !existing_enum_names.contains(token.name.as_str()))
        .map(|token| Literal::i32_unsuffixed(token.value))
        .collect()
}

fn syntax_kind_type() -> proc_macro2::TokenStream {
    quote! {
        ///
        ///  Kind of a `SyntaxKind`
        ///  This is the only manual definition required for properly creating a concrete
        /// syntax tree.
        ///  If a token is of type `Follow`, it is not immediately applied to the syntax
        /// tree, but put into
        ///  a buffer. Before the next node is started, all buffered tokens are applied
        /// to the syntax tree
        ///  at the depth of the node that is opened next.
        ///
        ///  For example, in `select * from contact;`, the whitespace between `*` and
        /// `from` should be a direct
        ///  child of the `SelectStmt` node. Without this concept, it would be put into
        /// the `ColumnRef`
        ///  node.
        ///
        ///  SelectStmt@0..22
        ///    Select@0..6 "select"
        ///    Whitespace@6..7 " "
        ///    ResTarget@7..8
        ///      ColumnRef@7..8
        ///        Ascii42@7..8 "*"
        ///    Whitespace@8..9 " "
        ///    From@9..13 "from"
        ///   Whitespace@13..14 " "
        ///    RangeVar@14..21
        ///      Ident@14..21 "contact"
        ///    Ascii59@21..22 ";"
        pub enum SyntaxKindType {
            Follow,
            Close,
        }
    }
}

fn syntax_kind_impl(
    node_identifiers: &[Ident],
    token_identifiers: &[Ident],
    token_value_literals: &[Literal],
) -> proc_macro2::TokenStream {
    let new_from_pg_query_node_fn = new_from_pg_query_node_fn(node_identifiers);
    let new_from_pg_query_token_fn =
        new_from_pg_query_token_fn(token_identifiers, token_value_literals);
    quote! {
        impl SyntaxKind {
            #new_from_pg_query_node_fn

            #new_from_pg_query_token_fn
        }
    }
}

fn new_from_pg_query_node_fn(node_identifiers: &[Ident]) -> proc_macro2::TokenStream {
    quote! {
        /// Converts a `pg_query` node to a `SyntaxKind`
        pub fn new_from_pg_query_node(node: &NodeEnum) -> Self {
            match node {
                #(NodeEnum::#node_identifiers(_) => SyntaxKind::#node_identifiers),*
            }
        }
    }
}

fn new_from_pg_query_token_fn(
    token_identifiers: &[Ident],
    token_value_literals: &[Literal],
) -> proc_macro2::TokenStream {
    quote! {
        /// Converts a `pg_query` token to a `SyntaxKind`
        pub fn new_from_pg_query_token(token: &ScanToken) -> Self {
            match token.token {
                #(#token_value_literals => SyntaxKind::#token_identifiers),*,
                _ => panic!("Unknown token"),
            }
        }
    }
}

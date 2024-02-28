use pest_derive::Parser;

/// The OWL2 Functional-style Syntax lexer.
#[derive(Debug, Parser)]
#[grammar = "grammars/bcp47.pest"]
#[grammar = "grammars/rfc3987.pest"]
#[grammar = "grammars/sparql.pest"]
#[grammar = "grammars/ofn.pest"]
pub struct OwlFunctionalLexer;
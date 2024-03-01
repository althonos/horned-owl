mod lexer;
mod from_pair;

use curie::PrefixMapping;
use crate::model::Build;
use crate::model::ForIRI;

// use self::from_pair::FromPair;
use self::lexer::OwlFunctionalLexer;
use self::lexer::Rule;

struct Context<'a, A: ForIRI> {
    build: &'a Build<A>,
    mapping: &'a PrefixMapping,
}

impl<'a, A: ForIRI> Context<'a, A> {
    fn new(build: &'a Build<A>, mapping: &'a PrefixMapping) -> Self {
        Self { build, mapping }
    }
}
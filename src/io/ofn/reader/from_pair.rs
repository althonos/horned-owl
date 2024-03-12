use std::collections::BTreeSet;
use std::str::FromStr;

use curie::Curie;
use curie::PrefixMapping;
use enum_meta::Meta;
use pest::iterators::Pair;

use crate::error::HornedError;
use crate::model::*;
use crate::ontology::set::SetOntology;
use crate::vocab::OWL2Datatype;
use crate::vocab::WithIRI;
use crate::vocab::OWL;

use super::Context;
use super::Rule;

// ---------------------------------------------------------------------------

type Result<T> = std::result::Result<T, HornedError>;

/// A trait for OWL elements that can be obtained from OWL Functional tokens.
///
/// `Pair<Rule>` values can be obtained from the `OwlFunctionalParser` struct
/// after parsing a document.
pub trait FromPair<A: ForIRI>: Sized {
    /// The valid production rule for the implementor.
    const RULE: Rule;

    /// Create a new instance from a `Pair`.
    #[inline]
    fn from_pair(pair: Pair<Rule>, context: &Context<'_, A>) -> Result<Self> {
        if cfg!(debug_assertions) && &pair.as_rule() != &Self::RULE {
            return Err(HornedError::from(pest::error::Error::new_from_span(
                pest::error::ErrorVariant::ParsingError {
                    positives: vec![pair.as_rule()],
                    negatives: vec![Self::RULE],
                },
                pair.as_span(),
            )));
        }
        Self::from_pair_unchecked(pair, context)
    }

    /// Create a new instance from a `Pair` without checking the PEG rule.
    fn from_pair_unchecked(pair: Pair<Rule>, context: &Context<'_, A>) -> Result<Self>;
}

// ---------------------------------------------------------------------------

macro_rules! impl_wrapper {
    ($ty:ident, $rule:path) => {
        impl<A: ForIRI> FromPair<A> for $ty<A> {
            const RULE: Rule = $rule;
            fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
                FromPair::from_pair(pair.into_inner().next().unwrap(), ctx).map($ty)
            }
        }
    };
}

impl_wrapper!(Class, Rule::Class);
impl_wrapper!(Import, Rule::Import);
impl_wrapper!(Datatype, Rule::Datatype);
impl_wrapper!(ObjectProperty, Rule::ObjectProperty);
impl_wrapper!(DataProperty, Rule::DataProperty);
impl_wrapper!(AnnotationProperty, Rule::AnnotationProperty);

impl_wrapper!(DeclareClass, Rule::ClassDeclaration);
impl_wrapper!(DeclareDatatype, Rule::DatatypeDeclaration);
impl_wrapper!(DeclareObjectProperty, Rule::ObjectPropertyDeclaration);
impl_wrapper!(DeclareDataProperty, Rule::DataPropertyDeclaration);
impl_wrapper!(
    DeclareAnnotationProperty,
    Rule::AnnotationPropertyDeclaration
);
impl_wrapper!(DeclareNamedIndividual, Rule::NamedIndividualDeclaration);

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for AnnotatedComponent<A> {
    const RULE: Rule = Rule::Axiom;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            // Declaration
            Rule::Declaration => {
                let mut inner = pair.into_inner();

                let ann = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let decl = inner.next().unwrap().into_inner().next().unwrap();
                let component = match decl.as_rule() {
                    Rule::ClassDeclaration => DeclareClass::from_pair(decl, ctx)?.into(),
                    Rule::DatatypeDeclaration => DeclareDatatype::from_pair(decl, ctx)?.into(),
                    Rule::ObjectPropertyDeclaration => {
                        DeclareObjectProperty::from_pair(decl, ctx)?.into()
                    }
                    Rule::DataPropertyDeclaration => {
                        DeclareDataProperty::from_pair(decl, ctx)?.into()
                    }
                    Rule::AnnotationPropertyDeclaration => {
                        DeclareAnnotationProperty::from_pair(decl, ctx)?.into()
                    }
                    Rule::NamedIndividualDeclaration => {
                        DeclareNamedIndividual::from_pair(decl, ctx)?.into()
                    }
                    rule => {
                        unreachable!(
                            "unexpected rule in AnnotatedComponent::Declaration: {:?}",
                            rule
                        )
                    }
                };

                Ok(Self { component, ann })
            }

            // ClassAxiom
            Rule::SubClassOf => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let subcls = ClassExpression::from_pair(inner.next().unwrap(), ctx)?;
                let supercls = ClassExpression::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(SubClassOf::new(supercls, subcls), annotations))
            }
            Rule::EquivalentClasses => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ce = inner
                    .map(|pair| FromPair::from_pair(pair, ctx))
                    .collect::<Result<_>>()?;
                Ok(Self::new(EquivalentClasses(ce), annotations))
            }
            Rule::DisjointClasses => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ce = inner
                    .map(|pair| FromPair::from_pair(pair, ctx))
                    .collect::<Result<_>>()?;
                Ok(Self::new(DisjointClasses(ce), annotations))
            }
            Rule::DisjointUnion => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let cls = Class::from_pair(inner.next().unwrap(), ctx)?;
                let ce = inner
                    .map(|pair| FromPair::from_pair(pair, ctx))
                    .collect::<Result<_>>()?;
                Ok(Self::new(DisjointUnion(cls, ce), annotations))
            }

            // ObjectPropertyAxiom
            Rule::SubObjectPropertyOf => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let sub = SubObjectPropertyExpression::from_pair(inner.next().unwrap(), ctx)?;
                let sup = ObjectPropertyExpression::from_pair(
                    inner.next().unwrap().into_inner().next().unwrap(),
                    ctx,
                )?;
                Ok(Self::new(SubObjectPropertyOf { sup, sub }, annotations))
            }
            Rule::EquivalentObjectProperties => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ops = inner
                    .map(|pair| FromPair::from_pair(pair, ctx))
                    .collect::<Result<_>>()?;
                Ok(Self::new(EquivalentObjectProperties(ops), annotations))
            }
            Rule::DisjointObjectProperties => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ops = inner
                    .map(|pair| FromPair::from_pair(pair, ctx))
                    .collect::<Result<_>>()?;
                Ok(Self::new(DisjointObjectProperties(ops), annotations))
            }
            Rule::ObjectPropertyDomain => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ope = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ce = ClassExpression::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(ObjectPropertyDomain::new(ope, ce), annotations))
            }
            Rule::ObjectPropertyRange => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ope = ObjectPropertyExpression::from_pair(inner.next().unwrap(), ctx)?;
                let ce = ClassExpression::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(ObjectPropertyRange::new(ope, ce), annotations))
            }
            Rule::InverseObjectProperties => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let r1 = ObjectProperty::from_pair(inner.next().unwrap(), ctx)?;
                let r2 = ObjectProperty::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(InverseObjectProperties(r1, r2), annotations))
            }
            Rule::FunctionalObjectProperty => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let r = ObjectPropertyExpression::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(FunctionalObjectProperty(r), annotations))
            }
            Rule::InverseFunctionalObjectProperty => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let r = ObjectPropertyExpression::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(InverseFunctionalObjectProperty(r), annotations))
            }
            Rule::ReflexiveObjectProperty => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let r = ObjectPropertyExpression::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(ReflexiveObjectProperty(r), annotations))
            }
            Rule::IrreflexiveObjectProperty => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let r = ObjectPropertyExpression::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(IrreflexiveObjectProperty(r), annotations))
            }
            Rule::SymmetricObjectProperty => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let r = ObjectPropertyExpression::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(SymmetricObjectProperty(r), annotations))
            }
            Rule::AsymmetricObjectProperty => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let r = ObjectPropertyExpression::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(AsymmetricObjectProperty(r), annotations))
            }
            Rule::TransitiveObjectProperty => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let r = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(TransitiveObjectProperty(r), annotations))
            }

            // DataPropertyAxiom
            Rule::SubDataPropertyOf => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let sub = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let sup = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(SubDataPropertyOf { sub, sup }, annotations))
            }
            Rule::EquivalentDataProperties => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let dps = inner
                    .map(|pair| FromPair::from_pair(pair, ctx))
                    .collect::<Result<_>>()?;
                Ok(Self::new(EquivalentDataProperties(dps), annotations))
            }
            Rule::DisjointDataProperties => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let dps = inner
                    .map(|pair| FromPair::from_pair(pair, ctx))
                    .collect::<Result<_>>()?;
                Ok(Self::new(DisjointDataProperties(dps), annotations))
            }
            Rule::DataPropertyDomain => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let dp = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ce = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(DataPropertyDomain::new(dp, ce), annotations))
            }
            Rule::DataPropertyRange => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let dp = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ce = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(DataPropertyRange::new(dp, ce), annotations))
            }
            Rule::FunctionalDataProperty => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let dp = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(FunctionalDataProperty(dp), annotations))
            }
            Rule::DatatypeDefinition => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let k = Datatype::from_pair(inner.next().unwrap(), ctx)?;
                let r = DataRange::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(DatatypeDefinition::new(k, r), annotations))
            }

            // HasKey
            Rule::HasKey => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ce = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let vpe = inner
                    .map(|pair| match pair.as_rule() {
                        Rule::ObjectPropertyExpression => FromPair::from_pair(pair, ctx)
                            .map(PropertyExpression::ObjectPropertyExpression),
                        Rule::DataProperty => {
                            FromPair::from_pair(pair, ctx).map(PropertyExpression::DataProperty)
                        }
                        _ => unreachable!(),
                    })
                    .collect::<Result<_>>()?;
                Ok(Self::new(HasKey::new(ce, vpe), annotations))
            }

            // Assertion
            Rule::SameIndividual => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let individuals = inner
                    .map(|pair| Individual::from_pair(pair, ctx))
                    .collect::<Result<_>>()?;
                Ok(Self::new(SameIndividual(individuals), annotations))
            }
            Rule::DifferentIndividuals => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let individuals = inner
                    .map(|pair| Individual::from_pair(pair, ctx))
                    .collect::<Result<_>>()?;
                Ok(Self::new(DifferentIndividuals(individuals), annotations))
            }
            Rule::ClassAssertion => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ce = ClassExpression::from_pair(inner.next().unwrap(), ctx)?;
                let i = Individual::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(ClassAssertion::new(ce, i), annotations))
            }
            Rule::ObjectPropertyAssertion => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ope = ObjectPropertyExpression::from_pair(inner.next().unwrap(), ctx)?;
                let from = Individual::from_pair(inner.next().unwrap(), ctx)?;
                let to = Individual::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(
                    ObjectPropertyAssertion { ope, from, to },
                    annotations,
                ))
            }
            Rule::NegativeObjectPropertyAssertion => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ope = ObjectPropertyExpression::from_pair(inner.next().unwrap(), ctx)?;
                let from = Individual::from_pair(inner.next().unwrap(), ctx)?.into();
                let to = Individual::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(
                    NegativeObjectPropertyAssertion::new(ope, from, to),
                    annotations,
                ))
            }
            Rule::DataPropertyAssertion => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ope = DataProperty::from_pair(inner.next().unwrap(), ctx)?;
                let from = Individual::from_pair(inner.next().unwrap(), ctx)?;
                let to = Literal::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(
                    DataPropertyAssertion::new(ope, from, to),
                    annotations,
                ))
            }
            Rule::NegativeDataPropertyAssertion => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ope = DataProperty::from_pair(inner.next().unwrap(), ctx)?;
                let from = Individual::from_pair(inner.next().unwrap(), ctx)?;
                let to = Literal::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(
                    NegativeDataPropertyAssertion::new(ope, from, to),
                    annotations,
                ))
            }

            // AnnotationAxiom
            Rule::AnnotationAssertion => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ap = AnnotationProperty::from_pair(inner.next().unwrap(), ctx)?;
                let subject = AnnotationSubject::from_pair(inner.next().unwrap(), ctx)?;
                let av = AnnotationValue::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(
                    AnnotationAssertion::new(subject, Annotation { ap, av }),
                    annotations,
                ))
            }
            Rule::SubAnnotationPropertyOf => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let sub =
                    FromPair::from_pair(inner.next().unwrap().into_inner().next().unwrap(), ctx)?;
                let sup =
                    FromPair::from_pair(inner.next().unwrap().into_inner().next().unwrap(), ctx)?;
                Ok(Self::new(SubAnnotationPropertyOf { sub, sup }, annotations))
            }
            Rule::AnnotationPropertyDomain => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ap = AnnotationProperty::from_pair(inner.next().unwrap(), ctx)?;
                let iri = IRI::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(
                    AnnotationPropertyDomain::new(ap, iri),
                    annotations,
                ))
            }
            Rule::AnnotationPropertyRange => {
                let mut inner = pair.into_inner();
                let annotations = FromPair::from_pair(inner.next().unwrap(), ctx)?;
                let ap = AnnotationProperty::from_pair(inner.next().unwrap(), ctx)?;
                let iri = IRI::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Self::new(
                    AnnotationPropertyRange::new(ap, iri),
                    annotations,
                ))
            }

            _ => unreachable!("unexpected rule in AnnotatedAxiom::from_pair"),
        }
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for Annotation<A> {
    const RULE: Rule = Rule::Annotation;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let mut inner = pair.into_inner();
        let _annotations: BTreeSet<Annotation<A>> =
            FromPair::from_pair(inner.next().unwrap(), ctx)?;

        Ok(Annotation {
            ap: FromPair::from_pair(inner.next().unwrap(), ctx)?,
            av: FromPair::from_pair(inner.next().unwrap(), ctx)?,
        })
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for AnnotationSubject<A> {
    const RULE: Rule = Rule::AnnotationSubject;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::IRI => FromPair::from_pair(inner, ctx).map(AnnotationSubject::IRI),
            // .map(Individual::Named)?,
            Rule::AnonymousIndividual => {
                FromPair::from_pair(inner, ctx).map(AnnotationSubject::AnonymousIndividual)
            }
            rule => {
                unreachable!(
                    "unexpected rule in AnnotationSubject::from_pair: {:?}",
                    rule
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for AnnotationValue<A> {
    const RULE: Rule = Rule::AnnotationValue;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::IRI => IRI::from_pair(inner, ctx).map(AnnotationValue::IRI),
            Rule::Literal => Literal::from_pair(inner, ctx).map(AnnotationValue::Literal),
            Rule::AnonymousIndividual => {
                AnonymousIndividual::from_pair(inner, ctx).map(AnnotationValue::AnonymousIndividual)
            }
            _ => unreachable!(),
        }
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for AnonymousIndividual<A> {
    const RULE: Rule = Rule::AnonymousIndividual;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let nodeid = pair.into_inner().next().unwrap();
        let inner = nodeid.into_inner().next().unwrap();
        let iri = ctx.build.iri(inner.as_str());
        Ok(AnonymousIndividual(iri.underlying()))
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for Component<A> {
    const RULE: Rule = Rule::Axiom;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        AnnotatedComponent::from_pair_unchecked(pair, ctx).map(|ac| ac.component)
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for BTreeSet<Annotation<A>> {
    const RULE: Rule = Rule::Annotations;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        pair.into_inner()
            .map(|pair| Annotation::from_pair(pair, ctx))
            .collect()
    }
}

// ---------------------------------------------------------------------------

macro_rules! impl_ce_data_cardinality {
    ($ctx:ident, $inner:ident, $dt:ident) => {{
        let mut pair = $inner.into_inner();
        let n = u32::from_pair(pair.next().unwrap(), $ctx)?;
        let dp = DataProperty::from_pair(pair.next().unwrap(), $ctx)?;
        let dr = match pair.next() {
            Some(pair) => DataRange::from_pair(pair, $ctx)?,
            // No data range is equivalent to `rdfs:Literal` as a data range.
            // see https://www.w3.org/TR/owl2-syntax/#Data_Property_Cardinality_Restrictions
            None => Datatype($ctx.build.iri(OWL2Datatype::RDFSLiteral.iri_str())).into(),
        };
        Ok(ClassExpression::$dt { n, dp, dr })
    }};
}

macro_rules! impl_ce_obj_cardinality {
    ($ctx:ident, $inner:ident, $card:ident) => {{
        let mut pair = $inner.into_inner();
        let n = u32::from_pair(pair.next().unwrap(), $ctx)?;
        let ope = ObjectPropertyExpression::from_pair(pair.next().unwrap(), $ctx)?;
        let bce = match pair.next() {
            Some(x) => Self::from_pair(x, $ctx).map(Box::new)?,
            // Missing class expression is equivalent to `owl:Thing` as class expression.
            // see https://www.w3.org/TR/owl2-syntax/#Object_Property_Cardinality_Restrictions
            None => Box::new(ClassExpression::Class(Class(
                $ctx.build.iri(OWL::Thing.iri_str()),
            ))),
        };
        Ok(ClassExpression::$card { n, ope, bce })
    }};
}

impl<A: ForIRI> FromPair<A> for ClassExpression<A> {
    const RULE: Rule = Rule::ClassExpression;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::Class => Class::from_pair(inner, ctx).map(ClassExpression::Class),
            Rule::ObjectIntersectionOf => inner
                .into_inner()
                .map(|pair| Self::from_pair(pair, ctx))
                .collect::<Result<_>>()
                .map(ClassExpression::ObjectIntersectionOf),
            Rule::ObjectUnionOf => inner
                .into_inner()
                .map(|pair| Self::from_pair(pair, ctx))
                .collect::<Result<_>>()
                .map(ClassExpression::ObjectUnionOf),
            Rule::ObjectComplementOf => Self::from_pair(inner.into_inner().next().unwrap(), ctx)
                .map(Box::new)
                .map(ClassExpression::ObjectComplementOf),
            Rule::ObjectOneOf => inner
                .into_inner()
                .map(|pair| Individual::from_pair(pair, ctx))
                .collect::<Result<_>>()
                .map(ClassExpression::ObjectOneOf),
            Rule::ObjectSomeValuesFrom => {
                let mut pairs = inner.into_inner();
                let ope = ObjectPropertyExpression::from_pair(pairs.next().unwrap(), ctx)?;
                let bce = Self::from_pair(pairs.next().unwrap(), ctx).map(Box::new)?;
                Ok(ClassExpression::ObjectSomeValuesFrom { ope, bce })
            }
            Rule::ObjectAllValuesFrom => {
                let mut pairs = inner.into_inner();
                let ope = ObjectPropertyExpression::from_pair(pairs.next().unwrap(), ctx)?;
                let bce = Self::from_pair(pairs.next().unwrap(), ctx).map(Box::new)?;
                Ok(ClassExpression::ObjectAllValuesFrom { ope, bce })
            }
            Rule::ObjectHasValue => {
                let mut pairs = inner.into_inner();
                let ope = ObjectPropertyExpression::from_pair(pairs.next().unwrap(), ctx)?;
                let i = Individual::from_pair(pairs.next().unwrap(), ctx)?;
                Ok(ClassExpression::ObjectHasValue { ope, i })
            }
            Rule::ObjectHasSelf => {
                let pair = inner.into_inner().next().unwrap();
                let expr = ObjectPropertyExpression::from_pair(pair, ctx)?;
                Ok(ClassExpression::ObjectHasSelf(expr))
            }
            Rule::ObjectMinCardinality => {
                impl_ce_obj_cardinality!(ctx, inner, ObjectMinCardinality)
            }
            Rule::ObjectMaxCardinality => {
                impl_ce_obj_cardinality!(ctx, inner, ObjectMaxCardinality)
            }
            Rule::ObjectExactCardinality => {
                impl_ce_obj_cardinality!(ctx, inner, ObjectExactCardinality)
            }
            Rule::DataSomeValuesFrom => {
                let mut pair = inner.into_inner();
                let dp = DataProperty::from_pair(pair.next().unwrap(), ctx)?;
                let next = pair.next().unwrap();
                if next.as_rule() == Rule::DataProperty {
                    unimplemented!() // FIXME!!!
                                     // Err(Error::custom(
                                     //     "cannot use data property chaining in `DataSomeValuesFrom`",
                                     //     next.as_span(),
                                     // ))
                } else {
                    let dr = DataRange::from_pair(next, ctx)?;
                    Ok(ClassExpression::DataSomeValuesFrom { dp, dr })
                }
            }
            Rule::DataAllValuesFrom => {
                let mut pair = inner.into_inner();
                let dp = DataProperty::from_pair(pair.next().unwrap(), ctx)?;
                let next = pair.next().unwrap();
                if next.as_rule() == Rule::DataProperty {
                    unimplemented!() // FIXME!!!
                                     // Err(Error::custom(
                                     //     "cannot use data property chaining in `DataAllValuesFrom`",
                                     //     next.as_span(),
                                     // ))
                } else {
                    let dr = DataRange::from_pair(next, ctx)?;
                    Ok(ClassExpression::DataAllValuesFrom { dp, dr })
                }
            }
            Rule::DataHasValue => {
                let mut pair = inner.into_inner();
                let dp = DataProperty::from_pair(pair.next().unwrap(), ctx)?;
                let l = Literal::from_pair(pair.next().unwrap(), ctx)?;
                Ok(ClassExpression::DataHasValue { dp, l })
            }
            Rule::DataMinCardinality => {
                impl_ce_data_cardinality!(ctx, inner, DataMinCardinality)
            }
            Rule::DataMaxCardinality => {
                impl_ce_data_cardinality!(ctx, inner, DataMaxCardinality)
            }
            Rule::DataExactCardinality => {
                impl_ce_data_cardinality!(ctx, inner, DataExactCardinality)
            }
            rule => unreachable!("unexpected rule in ClassExpression::from_pair: {:?}", rule),
        }
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for DataRange<A> {
    const RULE: Rule = Rule::DataRange;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::Datatype => Datatype::from_pair(inner, ctx).map(DataRange::Datatype),
            Rule::DataIntersectionOf => inner
                .into_inner()
                .map(|pair| Self::from_pair(pair, ctx))
                .collect::<Result<_>>()
                .map(DataRange::DataIntersectionOf),
            Rule::DataUnionOf => inner
                .into_inner()
                .map(|pair| Self::from_pair(pair, ctx))
                .collect::<Result<_>>()
                .map(DataRange::DataUnionOf),
            Rule::DataComplementOf => Self::from_pair(inner.into_inner().next().unwrap(), ctx)
                .map(Box::new)
                .map(DataRange::DataComplementOf),
            Rule::DataOneOf => inner
                .into_inner()
                .map(|pair| Literal::from_pair(pair, ctx))
                .collect::<Result<_>>()
                .map(DataRange::DataOneOf),
            Rule::DatatypeRestriction => {
                let mut pairs = inner.into_inner();
                Ok(DataRange::DatatypeRestriction(
                    Datatype::from_pair(pairs.next().unwrap(), ctx)?,
                    pairs
                        .map(|pair| FacetRestriction::from_pair(pair, ctx))
                        .collect::<Result<_>>()?,
                ))
            }
            rule => unreachable!("unexpected rule in DataRange::from_pair: {:?}", rule),
        }
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for Facet {
    const RULE: Rule = Rule::ConstrainingFacet;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let pair = pair.into_inner().next().unwrap();
        let span = pair.as_span();
        let iri = IRI::from_pair(pair, ctx)?;
        Facet::all()
            .into_iter()
            .find(|facet| &iri.to_string() == facet.iri_str())
            .ok_or_else(|| HornedError::invalid_at("invalid facet", span))
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for FacetRestriction<A> {
    const RULE: Rule = Rule::FacetRestriction;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let mut inner = pair.into_inner();
        let f = Facet::from_pair(inner.next().unwrap(), ctx)?;
        let l = Literal::from_pair(inner.next().unwrap(), ctx)?;
        Ok(FacetRestriction { f, l })
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for Individual<A> {
    const RULE: Rule = Rule::Individual;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::NamedIndividual => NamedIndividual::from_pair(inner, ctx).map(Individual::Named),
            Rule::AnonymousIndividual => {
                AnonymousIndividual::from_pair(inner, ctx).map(Individual::Anonymous)
            }
            rule => unreachable!("unexpected rule in Individual::from_pair: {:?}", rule),
        }
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for IRI<A> {
    const RULE: Rule = Rule::IRI;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::AbbreviatedIRI => {
                let span = inner.as_span();
                let mut pname = inner.into_inner().next().unwrap().into_inner();
                let prefix = pname.next().unwrap().into_inner().next();
                let local = pname.next().unwrap();
                let curie = Curie::new(prefix.map(|p| p.as_str()), local.as_str());
                match ctx.mapping.expand_curie(&curie) {
                    Ok(s) => Ok(ctx.build.iri(s)),
                    Err(curie::ExpansionError::Invalid) => {
                        Err(HornedError::invalid_at("undefined prefix", span))
                    }
                    Err(curie::ExpansionError::MissingDefault) => {
                        Err(HornedError::invalid_at("missing default prefix", span))
                    }
                }
            }
            Rule::FullIRI => {
                let iri = inner.into_inner().next().unwrap();
                Ok(ctx.build.iri(iri.as_str()))
            }
            rule => unreachable!("unexpected rule in IRI::from_pair: {:?}", rule),
        }
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for NamedIndividual<A> {
    const RULE: Rule = Rule::NamedIndividual;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        IRI::from_pair(pair.into_inner().next().unwrap(), ctx).map(NamedIndividual)
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for Literal<A> {
    const RULE: Rule = Rule::Literal;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::Literal => Self::from_pair(pair.into_inner().next().unwrap(), ctx),
            Rule::TypedLiteral => {
                let mut inner = pair.into_inner();
                let literal = String::from_pair(inner.next().unwrap(), ctx)?;
                let dty = Datatype::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Literal::Datatype {
                    literal,
                    datatype_iri: dty.0,
                })
            }
            Rule::StringLiteralWithLanguage => {
                let mut inner = pair.into_inner();
                let literal = String::from_pair(inner.next().unwrap(), ctx)?;
                let lang = inner.next().unwrap().as_str()[1..].trim().to_string();
                Ok(Literal::Language { literal, lang })
            }
            Rule::StringLiteralNoLanguage => {
                let mut inner = pair.into_inner();
                let literal = String::from_pair(inner.next().unwrap(), ctx)?;
                Ok(Literal::Simple { literal })
            }
            rule => unreachable!("unexpected rule in Literal::from_pair: {:?}", rule),
        }
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for ObjectPropertyExpression<A> {
    const RULE: Rule = Rule::ObjectPropertyExpression;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::ObjectProperty => {
                ObjectProperty::from_pair(inner, ctx).map(ObjectPropertyExpression::ObjectProperty)
            }
            Rule::InverseObjectProperty => {
                ObjectProperty::from_pair(inner.into_inner().next().unwrap(), ctx)
                    .map(ObjectPropertyExpression::InverseObjectProperty)
            }
            rule => unreachable!(
                "unexpected rule in ObjectPropertyExpression::from_pair: {:?}",
                rule
            ),
        }
    }
}

// ---------------------------------------------------------------------------

macro_rules! impl_ontology {
    ($ty:ident) => {
        impl<A: ForIRI> FromPair<A> for $ty<A> {
            const RULE: Rule = Rule::Ontology;
            fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
                debug_assert!(pair.as_rule() == Rule::Ontology);
                let mut pairs = pair.into_inner();
                let mut pair = pairs.next().unwrap();

                let mut ontology = $ty::default();
                let mut ontology_id = OntologyID::default();

                // Parse ontology IRI and Version IRI if any
                if pair.as_rule() == Rule::OntologyIRI {
                    let inner = pair.into_inner().next().unwrap();
                    ontology_id.iri = Some(IRI::from_pair(inner, ctx)?);
                    pair = pairs.next().unwrap();
                    if pair.as_rule() == Rule::VersionIRI {
                        let inner = pair.into_inner().next().unwrap();
                        ontology_id.viri = Some(IRI::from_pair(inner, ctx)?);
                        pair = pairs.next().unwrap();
                    }
                }
                ontology.insert(ontology_id);


                // Process imports
                for p in pair.into_inner() {
                    ontology.insert(Import::from_pair(p, ctx)?);
                }

                // Process ontology annotations
                for pair in pairs.next().unwrap().into_inner() {
                    ontology.insert(OntologyAnnotation::from_pair(pair, ctx)?);
                }

                // Process axioms, ignore SWRL rules
                for pair in pairs.next().unwrap().into_inner() {
                    let inner = pair.into_inner().next().unwrap();
                    match inner.as_rule() {
                        // FIXME: SWRL rules are not supported for now
                        Rule::Rule | Rule::DGAxiom => (),
                        Rule::Axiom => {
                            ontology.insert(AnnotatedComponent::from_pair(inner, ctx)?);
                        }
                        rule => {
                            unreachable!("unexpected rule in Ontology::from_pair: {:?}", rule);
                        }
                    }
                }

                Ok(ontology)
            }
        }
    };
}

impl_ontology!(SetOntology);
// impl_ontology!(AxiomMappedOntology);

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for OntologyAnnotation<A> {
    const RULE: Rule = Rule::Annotation;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        Annotation::from_pair(pair, ctx).map(OntologyAnnotation)
    }
}

// ---------------------------------------------------------------------------

impl<A, O> FromPair<A> for (O, PrefixMapping)
where
    A: ForIRI,
    O: Ontology<A> + FromPair<A>,
{
    const RULE: Rule = Rule::OntologyDocument;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let mut pairs = pair.into_inner();

        // Build the prefix mapping and use it to build the ontology
        let mut prefixes = PrefixMapping::default();
        let mut inner = pairs.next().unwrap();
        while inner.as_rule() == Rule::PrefixDeclaration {
            let mut decl = inner.into_inner();
            let mut pname = decl.next().unwrap().into_inner();
            let iri = decl.next().unwrap().into_inner().next().unwrap();

            if let Some(prefix) = pname.next().unwrap().into_inner().next() {
                prefixes
                    .add_prefix(prefix.as_str(), iri.as_str())
                    .expect("grammar does not allow invalid prefixes");
            } else {
                prefixes.set_default(iri.as_str());
            }

            inner = pairs.next().unwrap();
        }

        let context = Context::new(ctx.build, &prefixes);
        O::from_pair(inner, &context).map(|ont| (ont, prefixes))
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for String {
    const RULE: Rule = Rule::QuotedString;
    fn from_pair_unchecked(pair: Pair<Rule>, _ctx: &Context<'_, A>) -> Result<Self> {
        let l = pair.as_str().len();
        let s = &pair.as_str()[1..l - 1];
        if s.contains(r"\\") || s.contains(r#"\""#) {
            Ok(s.replace(r"\\", r"\").replace(r#"\""#, r#"""#))
        } else {
            Ok(s.to_string())
        }
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for SubObjectPropertyExpression<A> {
    const RULE: Rule = Rule::SubObjectPropertyExpression;
    fn from_pair_unchecked(pair: Pair<Rule>, ctx: &Context<'_, A>) -> Result<Self> {
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::ObjectPropertyExpression => ObjectPropertyExpression::from_pair(inner, ctx)
                .map(SubObjectPropertyExpression::ObjectPropertyExpression),
            Rule::PropertyExpressionChain => {
                let mut objs = Vec::new();
                for pair in inner.into_inner() {
                    objs.push(ObjectPropertyExpression::from_pair(pair, ctx)?);
                }
                Ok(SubObjectPropertyExpression::ObjectPropertyChain(objs))
            }
            rule => unreachable!(
                "unexpected rule in SubObjectProperty::from_pair: {:?}",
                rule
            ),
        }
    }
}

// ---------------------------------------------------------------------------

impl<A: ForIRI> FromPair<A> for u32 {
    const RULE: Rule = Rule::NonNegativeInteger;
    fn from_pair_unchecked(pair: Pair<Rule>, _ctx: &Context<'_, A>) -> Result<Self> {
        Ok(Self::from_str(pair.as_str()).expect("cannot fail with the right rule"))
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    use super::*;
    use crate::io::ofn::reader::lexer::OwlFunctionalLexer;

    macro_rules! assert_parse_into {
        ($ty:ty, $rule:path, $build:ident, $prefixes:ident, $doc:expr, $expected:expr) => {
            let doc = $doc.trim();
            let ctx = Context::new(&$build, &$prefixes);
            match OwlFunctionalLexer::lex($rule, doc) {
                Ok(mut pairs) => {
                    let res = <$ty as FromPair<_>>::from_pair(pairs.next().unwrap(), &ctx);
                    assert_eq!(res.unwrap(), $expected);
                }
                Err(e) => panic!(
                    "parsing using {:?}:\n{}\nfailed with: {}",
                    $rule,
                    doc.trim(),
                    e
                ),
            }
        };
    }

    #[test]
    fn has_key() {
        let build = Build::default();
        let mut prefixes = PrefixMapping::default();
        prefixes
            .add_prefix("owl", "http://www.w3.org/2002/07/owl#")
            .unwrap();

        assert_parse_into!(
            AnnotatedComponent<String>,
            Rule::Axiom,
            build,
            prefixes,
            "HasKey( owl:Thing () (<http://www.example.com/issn>) )",
            AnnotatedComponent::from(HasKey::new(
                ClassExpression::Class(build.class("http://www.w3.org/2002/07/owl#Thing")),
                vec![PropertyExpression::DataProperty(
                    build.data_property("http://www.example.com/issn")
                )],
            ))
        );
    }

    #[test]
    fn declare_class() {
        let build = Build::default();
        let mut prefixes = PrefixMapping::default();
        prefixes
            .add_prefix("owl", "http://www.w3.org/2002/07/owl#")
            .unwrap();

        assert_parse_into!(
            DeclareClass<String>,
            Rule::ClassDeclaration,
            build,
            prefixes,
            "Class( owl:Thing )",
            DeclareClass(build.class("http://www.w3.org/2002/07/owl#Thing"))
        );

        assert_parse_into!(
            Component<String>,
            Rule::Axiom,
            build,
            prefixes,
            "Declaration(Class(owl:Thing))",
            Component::DeclareClass(DeclareClass(
                build.class("http://www.w3.org/2002/07/owl#Thing")
            ))
        );

        assert_parse_into!(
            AnnotatedComponent<String>,
            Rule::Axiom,
            build,
            prefixes,
            "Declaration(Class(owl:Thing))",
            AnnotatedComponent::from(DeclareClass(
                build.class("http://www.w3.org/2002/07/owl#Thing")
            ))
        );
    }

    #[test]
    fn iri() {
        let build = Build::default();
        let mut prefixes = PrefixMapping::default();
        prefixes
            .add_prefix("ex", "http://example.com/path#")
            .unwrap();

        assert_parse_into!(
            IRI<String>,
            Rule::IRI,
            build,
            prefixes,
            "<http://example.com/path#ref>",
            build.iri("http://example.com/path#ref")
        );

        assert_parse_into!(
            IRI<String>,
            Rule::IRI,
            build,
            prefixes,
            "ex:ref",
            build.iri("http://example.com/path#ref")
        );
    }

    #[test]
    fn ontology_document() {
        let build = Build::default();
        let prefixes = PrefixMapping::default();
        let txt = "Prefix(ex:=<http://example.com/>) Prefix(:=<http://default.com/>) Ontology()";

        let mut expected = PrefixMapping::default();
        expected.set_default("http://default.com/");
        expected.add_prefix("ex", "http://example.com/").unwrap();

        let pair = OwlFunctionalLexer::lex(Rule::OntologyDocument, txt)
            .unwrap()
            .next()
            .unwrap();

        let doc: (SetOntology<String>, PrefixMapping) =
            FromPair::from_pair(pair, &Context::new(&build, &prefixes)).unwrap();
        assert_eq!(
            doc.1.mappings().collect::<HashSet<_>>(),
            expected.mappings().collect::<HashSet<_>>()
        );
    }

    macro_rules! test_from_pair {
        ($name:ident, $file:literal) => {
            #[test]
            fn $name() {
                let ont_s = include_str!(concat!("../../../ont/owl-functional/", $file));
                let pair = match OwlFunctionalLexer::lex(Rule::OntologyDocument, ont_s.trim()) {
                    Err(e) => panic!("parser failed: {}", e),
                    Ok(mut pairs) => {
                        let pair = pairs.next().unwrap();
                        assert_eq!(pair.as_str(), ont_s.trim());
                        pair
                    }
                };

                let build = Build::default();
                let prefixes = PrefixMapping::default();
                let ctx = Context::new(&build, &prefixes);
                let item: (SetOntology<String>, _) = FromPair::from_pair(pair, &ctx).unwrap();
            }
        };
    }

    macro_rules! generate_tests {
        ( $( $name:ident ( $file:literal ) ),* ) => {
            $( test_from_pair!($name, $file); )*
        }
    }

    generate_tests!(
        and_complex("and-complex.ofn"),
        and("and.ofn"),
        annotation_domain("annotation-domain.ofn"),
        annotation_iri("annotation-iri.ofn"),
        annotation_on_complex_subclass("annotation-on-complex-subclass.ofn"),
        annotation_on_subclass("annotation-on-subclass.ofn"),
        annotation_on_transitive("annotation-on-transitive.ofn"),
        annotation_property("annotation-property.ofn"),
        annotation_range("annotation-range.ofn"),
        annotation_with_annotation("annotation-with-annotation.ofn"),
        annotation_with_anonymous("annotation-with-anonymous.ofn"),
        annotation_with_non_builtin_annotation("annotation-with-non-builtin-annotation.ofn"),
        annotation("annotation.ofn"),
        annotation_assertion("annotation_assertion.ofn"),
        anon_subobjectproperty("anon-subobjectproperty.ofn"),
        anonymous_annotation_value("anonymous-annotation-value.ofn"),
        anonymous_individual("anonymous_individual.ofn"),
        class_assertion("class-assertion.ofn"),
        class("class.ofn"),
        class_with_two_annotations("class_with_two_annotations.ofn"),
        complex_equivalent_classes("complex-equivalent-classes.ofn"),
        data_exact_cardinality("data-exact-cardinality.ofn"),
        data_has_key("data-has-key.ofn"),
        data_has_value("data-has-value.ofn"),
        data_max_cardinality("data-max-cardinality.ofn"),
        data_min_cardinality("data-min-cardinality.ofn"),
        data_only("data-only.ofn"),
        data_property_assertion("data-property-assertion.ofn"),
        data_property_disjoint("data-property-disjoint.ofn"),
        data_property_domain("data-property-domain.ofn"),
        data_property_equivalent("data-property-equivalent.ofn"),
        data_property_functional("data-property-functional.ofn"),
        data_property_range("data-property-range.ofn"),
        data_property_sub("data-property-sub.ofn"),
        data_property("data-property.ofn"),
        data_some("data-some.ofn"),
        data_unqualified_exact("data-unqualified-exact.ofn"),
        datatype_alias("datatype-alias.ofn"),
        datatype_complement("datatype-complement.ofn"),
        datatype_definition("datatype-definition.ofn"),
        datatype_intersection("datatype-intersection.ofn"),
        datatype_oneof("datatype-oneof.ofn"),
        datatype_union("datatype-union.ofn"),
        datatype("datatype.ofn"),
        declaration_with_annotation("declaration-with-annotation.ofn"),
        declaration_with_two_annotation("declaration-with-two-annotation.ofn"),
        different_individual("different-individual.ofn"),
        different_individuals("different-individuals.ofn"),
        disjoint_class("disjoint-class.ofn"),
        disjoint_object_properties("disjoint-object-properties.ofn"),
        disjoint_union("disjoint-union.ofn"),
        equivalent_class("equivalent-class.ofn"),
        equivalent_object_properties("equivalent-object-properties.ofn"),
        equivalent_classes("equivalent_classes.ofn"),
        facet_restriction_complex("facet-restriction-complex.ofn"),
        facet_restriction("facet-restriction.ofn"),
        family_other("family-other.ofn"),
        family("family.ofn"),
        gci_and_other_class_relations("gci_and_other_class_relations.ofn"),
        happy_person("happy_person.ofn"),
        import_property("import-property.ofn"),
        import("import.ofn"),
        intersection("intersection.ofn"),
        inverse_properties("inverse-properties.ofn"),
        inverse_transitive("inverse-transitive.ofn"),
        label("label.ofn"),
        multi_different_individual("multi-different-individual.ofn"),
        multi_different_individuals("multi-different-individuals.ofn"),
        multi_has_key("multi-has-key.ofn"),
        multi_same_individual("multi-same-individual.ofn"),
        named_individual("named-individual.ofn"),
        negative_data_property_assertion("negative-data-property-assertion.ofn"),
        negative_object_property_assertion("negative-object-property-assertion.ofn"),
        not("not.ofn"),
        o10("o10.ofn"),
        object_exact_cardinality("object-exact-cardinality.ofn"),
        object_has_key("object-has-key.ofn"),
        object_has_self("object-has-self.ofn"),
        object_has_value("object-has-value.ofn"),
        object_max_cardinality("object-max-cardinality.ofn"),
        object_min_cardinality("object-min-cardinality.ofn"),
        object_one_of("object-one-of.ofn"),
        object_property_assertion("object-property-assertion.ofn"),
        object_property_asymmetric("object-property-asymmetric.ofn"),
        object_property_domain("object-property-domain.ofn"),
        object_property_functional("object-property-functional.ofn"),
        object_property_inverse_functional("object-property-inverse-functional.ofn"),
        object_property_irreflexive("object-property-irreflexive.ofn"),
        object_property_range("object-property-range.ofn"),
        object_property_reflexive("object-property-reflexive.ofn"),
        object_property_symmetric("object-property-symmetric.ofn"),
        object_unqualified_max_cardinality("object-unqualified-max-cardinality.ofn"),
        one_class_fully_qualified("one-class-fully-qualified.ofn"),
        one_class("one-class.ofn"),
        one_comment("one-comment.ofn"),
        one_ont_from_horned("one-ont-from-horned.ofn"),
        one_ontology_annotation("one-ontology-annotation.ofn"),
        one_or("one-or.ofn"),
        one_subclass("one-subclass.ofn"),
        only("only.ofn"),
        ont_with_bfo("ont-with-bfo.ofn"),
        ont("ont.ofn"),
        ontology_annotation("ontology-annotation.ofn"),
        oproperty("oproperty.ofn"),
        or("or.ofn"),
        other_iri("other-iri.ofn"),
        other_property("other-property.ofn"),
        other("other.ofn"),
        recursing_class("recursing_class.ofn"),
        same_individual("same-individual.ofn"),
        some_inverse("some-inverse.ofn"),
        some_not("some-not.ofn"),
        some("some.ofn"),
        sub_annotation("sub-annotation.ofn"),
        subclass("subclass.ofn"),
        subclasses_undeclared("subclasses-undeclared.ofn"),
        suboproperty_inverse("suboproperty-inverse.ofn"),
        suboproperty_top("suboproperty-top.ofn"),
        suboproperty("suboproperty.ofn"),
        subproperty_chain_with_inverse("subproperty-chain-with-inverse.ofn"),
        subproperty_chain("subproperty-chain.ofn"),
        subproperty("subproperty.ofn"),
        transitive_properties("transitive-properties.ofn"),
        two_annotation_on_transitive("two-annotation-on-transitive.ofn"),
        two_class_with_some("two-class-with-some.ofn"),
        two_class_with_subclass("two-class-with-subclass.ofn"),
        type_complex("type-complex.ofn"),
        type_individual_datatype_unqualified("type-individual-datatype-unqualified.ofn"),
        type_individual_datatype("type-individual-datatype.ofn"),
        typed_individual_datatype_unqualified("typed-individual-datatype-unqualified.ofn")
    );
}

//! Search facilities for Ontologies
//!
//! This library provides simple search facilities.
//!
//! It is currently being deprecated in favour of [`OntologyIndex`](../ontology/indexed/OntologyIndex.html)
use crate::ontology::axiom_mapped::AxiomMappedOntology;

use crate::model::*;

pub fn find_logically_equal_axiom<'a>(
    o: &'a AxiomMappedOntology,
    axiom: &AnnotatedAxiom,
) -> Option<&'a AnnotatedAxiom> {
    // Find any axiom in Ontology which is the same as AnnotatedAxiom,
    // ignoring the Annotations
    let o: &AxiomMappedOntology = o.into();
    o.i()
        .annotated_axiom(axiom.kind())
        .find(|ax| ax.logical_eq(axiom))
}

// Find an axiom which is logically equal and merge it's annotations
pub fn update_logically_equal_axiom<'a>(o: &mut AxiomMappedOntology, mut axiom: AnnotatedAxiom) {
    let some_eq_axiom = find_logically_equal_axiom(o, &axiom);

    if let Some(eq_axiom) = some_eq_axiom.cloned() {
        let mut taken_axiom = o.take(&eq_axiom).unwrap();
        axiom.ann.append(&mut taken_axiom.ann);
    }

    o.insert(axiom);
}

pub fn find_declaration_kind<'a>(o: &AxiomMappedOntology, iri: &IRI) -> Option<NamedEntityKind> {
    match 10 {
        _ if find_logically_equal_axiom(o, &DeclareClass(Class(iri.clone())).into()).is_some() => {
            return Some(NamedEntityKind::Class)
        }
        _ if find_logically_equal_axiom(
            o,
            &DeclareObjectProperty(ObjectProperty(iri.clone())).into(),
        )
        .is_some() =>
        {
            return Some(NamedEntityKind::ObjectProperty)
        }
        _ if find_logically_equal_axiom(
            o,
            &DeclareAnnotationProperty(AnnotationProperty(iri.clone())).into(),
        )
        .is_some() =>
        {
            return Some(NamedEntityKind::AnnotationProperty)
        }
        _ if find_logically_equal_axiom(
            o,
            &DeclareDataProperty(DataProperty(iri.clone())).into(),
        )
        .is_some() =>
        {
            return Some(NamedEntityKind::DataProperty)
        }
        _ if find_logically_equal_axiom(
            o,
            &DeclareNamedIndividual(NamedIndividual(iri.clone())).into(),
        )
        .is_some() =>
        {
            return Some(NamedEntityKind::NamedIndividual)
        }
        _ if find_logically_equal_axiom(o, &DeclareDatatype(Datatype(iri.clone())).into())
            .is_some() =>
        {
            return Some(NamedEntityKind::Datatype)
        }
        _ => {
            return crate::vocab::to_built_in_entity(iri);
        }
    }
}

pub fn is_annotation_property(o: &AxiomMappedOntology, iri: &IRI) -> bool {
    match find_declaration_kind(o, iri) {
        Some(NamedEntityKind::AnnotationProperty) => true,
        _ => false,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    //use crate::model::*;

    #[test]
    fn test_find_equal_axiom() {
        let b = Build::new();
        let mut o = AxiomMappedOntology::default();

        let c = b.class("http://www.example.com");
        o.declare(c);

        let ne: NamedEntity = b.class("http://www.example.com").into();
        let ax: Axiom = ne.into();
        let dec: AnnotatedAxiom = ax.into();

        let flea = find_logically_equal_axiom(&o, &dec);
        assert!(flea.is_some());

        let flea = flea.unwrap();
        assert_eq!(flea.kind(), AxiomKind::DeclareClass);

        if let Axiom::DeclareClass(ref dc) = flea.axiom {
            assert_eq!(dc.0, b.class("http://www.example.com"));
        }
    }

    #[test]
    fn test_update_equal_axiom() {
        let b = Build::new();
        {
            let mut o = AxiomMappedOntology::default();
            let ne: NamedEntity = b.class("http://www.example.com").into();
            let ax: Axiom = ne.into();
            let mut dec: AnnotatedAxiom = ax.into();

            dec.ann.insert(Annotation {
                ap: b.annotation_property("http://www.example.com/p1"),
                av: b.iri("http://www.example.com/a1").into(),
            });

            let ne: NamedEntity = b.class("http://www.example.com").into();
            let ax: Axiom = ne.into();
            let mut dec2: AnnotatedAxiom = ax.into();

            dec2.ann.insert(Annotation {
                ap: b.annotation_property("http://www.example.com/p1"),
                av: b.iri("http://www.example.com/a2").into(),
            });

            o.insert(dec);
            o.insert(dec2);
            assert_eq!(o.i().iter().count(), 2);
        }

        {
            let mut o = AxiomMappedOntology::default();
            let ne: NamedEntity = b.class("http://www.example.com").into();
            let ax: Axiom = ne.into();
            let mut dec: AnnotatedAxiom = ax.into();
            dec.ann.insert(Annotation {
                ap: b.annotation_property("http://www.example.com/p1"),
                av: b.iri("http://www.example.com/a1").into(),
            });

            let ne: NamedEntity = b.class("http://www.example.com").into();
            let ax: Axiom = ne.into();
            let mut dec2: AnnotatedAxiom = ax.into();
            dec2.ann.insert(Annotation {
                ap: b.annotation_property("http://www.example.com/p1"),
                av: b.iri("http://www.example.com/a2").into(),
            });

            o.insert(dec);
            update_logically_equal_axiom(&mut o, dec2);
            assert_eq!(o.i().iter().count(), 1);

            let aa = o.i().iter().next().unwrap();

            assert_eq!(aa.ann.iter().count(), 2);
        }
    }

    #[test]
    fn test_find_declaration_single() {
        let b = Build::new();
        let mut o = AxiomMappedOntology::default();

        o.declare(b.class("http://www.example.com/c"));
        o.declare(b.object_property("http://www.example.com/ob"));

        assert_eq!(
            find_declaration_kind(&o, &b.iri("http://www.example.com/c")),
            Some(NamedEntityKind::Class)
        );

        assert_eq!(
            find_declaration_kind(&o, &b.iri("http://www.example.com/ob")),
            Some(NamedEntityKind::ObjectProperty)
        );

        assert_eq!(
            find_declaration_kind(&o, &b.iri("http://www.example.com/fred")),
            None
        );
    }
}

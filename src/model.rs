#![allow(dead_code)]

use std::collections::HashSet;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;

#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IRI(Rc<String>);

impl Deref for IRI{
    type Target = String;

    fn deref(&self) -> &String{
        &self.0
    }
}

#[test]
fn test_iri_from_string() {
    let iri_build = IRIBuild::new();
    let iri = iri_build.iri("http://www.example.com");

    assert_eq!(String::from(iri), "http://www.example.com");
}


impl From<IRI> for String{
    fn from(i:IRI) -> String {
        // Clone Rc'd value
        (*i.0).clone()
    }
}

#[derive(Debug)]
pub struct IRIBuild(Rc<RefCell<HashSet<IRI>>>);

impl IRIBuild{
    pub fn new() -> IRIBuild{
        IRIBuild(Rc::new(RefCell::new(HashSet::new())))
    }

    pub fn iri<S>(&self, s: S) -> IRI
        where S: Into<String>
    {
        let iri = IRI(Rc::new(s.into()));

        let mut cache = self.0.borrow_mut();
        if cache.contains(&iri){
            return cache.get(&iri).unwrap().clone()
        }

        cache.insert(iri.clone());
        return iri;
    }
}

#[test]
fn test_iri_creation(){
    let iri_build = IRIBuild::new();

    let iri1 = iri_build.iri("http://example.com".to_string());

    let iri2 = iri_build.iri("http://example.com".to_string());

    // these are equal to each other
    assert_eq!(iri1, iri2);

    // these are the same object in memory
    assert!(Rc::ptr_eq(&iri1.0, &iri2.0));

    // iri1, iri2 and one in the cache == 3
    assert_eq!(Rc::strong_count(&iri1.0), 3);
}

#[test]
fn test_iri_string_creation(){
    let iri_build = IRIBuild::new();

    let iri_string = iri_build.iri("http://www.example.com".to_string());
    let iri_static = iri_build.iri("http://www.example.com");
    let iri_from_iri = iri_build.iri(iri_static.clone());

    let s = "http://www.example.com";
    let iri_str = iri_build.iri(&s[..]);

    assert_eq!(iri_string, iri_static);
    assert_eq!(iri_string, iri_str);
    assert_eq!(iri_static, iri_str);
    assert_eq!(iri_from_iri, iri_str);
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Class(pub IRI);

impl From<Class> for IRI {
    fn from(c: Class) -> IRI {
        Self::from(&c)
    }
}

impl <'a> From<&'a Class> for IRI {
    fn from(c: &Class) -> IRI {
        (c.0).clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct ObjectProperty(pub IRI);

impl From<ObjectProperty> for IRI {
    fn from(c: ObjectProperty) -> IRI {
        Self::from(&c)
    }
}

impl <'a> From<&'a ObjectProperty> for IRI {
    fn from(c: &ObjectProperty) -> IRI {
        (c.0).clone()
    }
}


#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub enum NamedEntity {
    Class(Class),
    ObjectProperty(ObjectProperty)
}

#[derive(Eq,PartialEq,Hash,Clone,Debug)]
pub struct SubClass{
    pub superclass: ClassExpression,
    pub subclass: ClassExpression,
}

#[derive(Eq,PartialEq,Hash,Clone,Debug)]
pub enum ClassExpression
{
    Class(Class),
    Some{o:ObjectProperty, ce:Box<ClassExpression>},
    Only{o:ObjectProperty, ce:Box<ClassExpression>},
    And{o:Vec<ClassExpression>},
    Or{o:Vec<ClassExpression>},
    Not{ce:Box<ClassExpression>}
}

#[derive(Debug, Eq, PartialEq)]
pub struct OntologyID{
    pub iri: Option<IRI>,
    pub viri: Option<IRI>,
}

#[derive(Debug)]
pub struct Ontology
{
    pub iri_build:IRIBuild,
    pub id: OntologyID,
    pub class: HashSet<Class>,
    pub subclass: HashSet<SubClass>,
    pub object_property: HashSet<ObjectProperty>,

}

impl PartialEq for Ontology {
    fn eq(&self, other: &Ontology) -> bool {
        self.id == other.id &&
            self.class == other.class &&
            self.subclass == other.subclass &&
            self.object_property == other.object_property
    }
}

impl Eq for Ontology {}

impl Ontology {
    pub fn new() -> Ontology{
        Ontology::new_with_build(IRIBuild::new())
    }

    pub fn new_with_build(iri_build:IRIBuild) -> Ontology{
        Ontology{
            iri_build: iri_build,
            id: OntologyID{iri:None,viri:None},
            class: HashSet::new(),
            subclass: HashSet::new(),
            object_property: HashSet::new(),
        }
    }

    /// Constructs a new `IRI`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use horned_owl::model::*;
    /// let mut o = Ontology::new();
    /// let iri = o.iri("http://www.example.com".to_string());
    /// let iri2 = o.iri("http://www.example.com");
    ///
    /// assert_eq!(iri, iri2);
    /// ```
    pub fn iri<S>(&self, s: S)-> IRI
        where S: Into<String> {
        self.iri_build.iri(s)
    }

    /// Constructs a new `Class` from an existing IRI. This is
    /// slightly more efficient that using `class`, when an IRI has
    /// already been created.
    ///
    /// # Examples
    ///
    /// ```
    /// # use horned_owl::model::*;
    /// let mut o = Ontology::new();
    /// let iri = o.class("http://www.example.com".to_string());
    /// let iri2 = o.class("http://www.example.com");
    ///
    /// assert_eq!(iri, iri2);
    /// ```
    ///
    pub fn class_from_iri(&mut self, i: IRI) -> Class {
        let c = Class(i);

        if let Option::Some(_) = self.class.get(&c)
        {return c;}

        self.class.insert(c.clone());
        c
    }

    /// Constructs a new `Class`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use horned_owl::model::*;
    /// let mut o = Ontology::new();
    /// let iri = o.class("http://www.example.com".to_string());
    /// let iri2 = o.class("http://www.example.com");
    ///
    /// assert_eq!(iri, iri2);
    /// ```
    ///
    pub fn class<S>(&mut self, s: S) -> Class
        where S: Into<String>
    {
        let i = self.iri(s);
        self.class_from_iri(i)
    }

    pub fn object_property_from_iri(&mut self, i: IRI) -> ObjectProperty
    {
        let o = ObjectProperty(i);

        if let Option::Some(_) = self.object_property.get(&o)
        {return o;};

        self.object_property.insert(o.clone());
        o
    }

    /// Constructs a new `ObjectProperty`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use horned_owl::model::*;
    /// let mut o = Ontology::new();
    /// let iri = o.object_property("http://www.example.com".to_string());
    /// let iri2 = o.object_property("http://www.example.com");
    ///
    /// assert_eq!(iri, iri2);
    /// ```
    pub fn object_property<S>(&mut self, s:S) -> ObjectProperty
        where S: Into<String>
    {
        let i = self.iri(s);
        self.object_property_from_iri(i)
    }

    pub fn named_entity(&mut self, ne: NamedEntity)
    {
        match ne {
            NamedEntity::Class(c) => {
                self.class_from_iri(c.0);
            }
            NamedEntity::ObjectProperty(i) => {
                self.object_property_from_iri(i.0);
            }
        }
    }

    /// Adds a subclass axiom to the ontology
    ///
    /// # Examples
    ///
    /// ```
    /// # use horned_owl::model::*;
    /// let mut o = Ontology::new();
    /// let sup = o.class("http://www.example.com/super");
    /// let sub = o.class("http://www.example.com/sub");
    ///
    /// o.subclass(sup, sub);
    /// ```
    pub fn subclass(&mut self, superclass:Class, subclass: Class)
                    -> SubClass
    {
        self.subclass_exp(ClassExpression::Class(superclass),
                          ClassExpression::Class(subclass))
    }

    pub fn subclass_exp(&mut self, superclass:ClassExpression,
                        subclass: ClassExpression) -> SubClass
    {
        let sc = SubClass{superclass:superclass,subclass:subclass};

        if let Some(_) = self.subclass.get(&sc)
        {return sc;}

        self.subclass.insert(sc.clone());
        sc
    }

    /// Returns all direct subclasses
    ///
    /// # Examples
    ///
    /// ```
    /// # use horned_owl::model::*;
    /// let mut o = Ontology::new();
    /// let sup = o.class("http://www.example.com/super");
    /// let sub = o.class("http://www.example.com/sub");
    /// let subsub = o.class("http://www.example.com/subsub");
    ///
    /// o.subclass(sup.clone(), sub.clone());
    /// o.subclass(sub.clone(), subsub);
    ///
    /// let subs = o.direct_subclass(&sup);
    ///
    /// assert_eq!(vec![&ClassExpression::Class(sub)],subs);
    /// ```
    pub fn direct_subclass(&self, c: &Class)
                           ->Vec<&ClassExpression>{
        let ce = ClassExpression::Class(c.clone());
        self.direct_subclass_exp(&ce)
    }

    pub fn direct_subclass_exp(&self, c: &ClassExpression)
                           -> Vec<&ClassExpression>{
        self.subclass
            .iter()
            .filter(|sc| &sc.superclass == c )
            .map(|sc| &sc.subclass )
            .collect::<Vec<&ClassExpression>>()
    }

    /// Returns true is `subclass` is a subclass of `superclass`
    ///
    /// # Examples
    ///
    /// ```
    /// # use horned_owl::model::*;
    /// let mut o = Ontology::new();
    /// let sup = o.class("http://www.example.com/super");
    /// let sub = o.class("http://www.example.com/sub");
    /// let subsub = o.class("http://www.example.com/subsub");
    ///
    /// o.subclass(sup.clone(), sub.clone());
    /// o.subclass(sub.clone(), subsub.clone());
    ///
    /// assert!(o.is_subclass(&sup, &sub));
    /// assert!(!o.is_subclass(&sub, &sup));
    /// assert!(!o.is_subclass(&sup, &subsub));
    /// ```
    pub fn is_subclass(&self, superclass:&Class,
                       subclass:&Class) -> bool {
        self.is_subclass_exp(&ClassExpression::Class(superclass.clone()),
                             &ClassExpression::Class(subclass.clone()))
    }

    pub fn is_subclass_exp(&self, superclass:&ClassExpression,
                           subclass:&ClassExpression)
        -> bool {

        let first:Option<&SubClass> =
            self.subclass.iter()
            .filter(|sc|
                    sc.superclass == *superclass &&
                    sc.subclass == *subclass)
            .next();

        match first
        {
            Option::Some(_) => true,
            None => false
        }
    }
}

#[cfg(test)]
mod test{
    use super::*;

    #[test]
    fn test_ontology_cons(){
        let _ = Ontology::new();
        assert!(true);
    }

    #[test]
    fn test_iri(){
        let o = Ontology::new();
        let iri = o.iri("http://www.example.com".to_string());
        assert_eq!(*iri.0, "http://www.example.com".to_string());
    }

    #[test]
    fn test_class(){
        let mut o = Ontology::new();
        let iri = o.iri("http://www.example.com".to_string());

        let a = o.class("http://www.example.com");
        let b = o.class(iri);
        assert_eq!(a,b);
    }

    #[test]
    fn test_class_iri(){
        let mut o = Ontology::new();

        let iri = o.iri("http://www.example.com".to_string());
        let a = o.class(iri.clone());
        let b = o.class_from_iri(iri);

        assert_eq!(a,b);
    }

    #[test]
    fn test_object_property(){
        let mut o = Ontology::new();
        let iri = o.iri("http://www.example.com".to_string());

        let a = o.object_property(iri.clone());
        let b = o.object_property(iri);
        assert_eq!(a,b);
    }

}

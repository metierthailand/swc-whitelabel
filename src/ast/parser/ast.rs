#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForModifier {
    For(String),
    Wildcard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modifier {
    Optional(bool),
    ForModifier(ForModifier),
    Key(String),
}

pub type Directive = Vec<Modifier>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectiveError {
    ForOrWildcardConflict,
}

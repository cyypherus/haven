use super::MouseButton;
use crate::{Key, Modifier, Modifiers, NamedKey};
use std::ops::{BitAnd, BitOr, Not};

#[derive(Debug, Clone)]
struct Expr<T> {
    nodes: Vec<ExprNode<T>>,
    root: usize,
}

#[derive(Debug, Clone)]
enum ExprNode<T> {
    Any,
    Atom(T),
    Not(usize),
    And(usize, usize),
    Or(usize, usize),
}

impl<T> ExprNode<T> {
    fn offset(self, offset: usize) -> Self {
        match self {
            Self::Any => Self::Any,
            Self::Atom(value) => Self::Atom(value),
            Self::Not(inner) => Self::Not(inner + offset),
            Self::And(left, right) => Self::And(left + offset, right + offset),
            Self::Or(left, right) => Self::Or(left + offset, right + offset),
        }
    }
}

impl<T> Expr<T> {
    fn any() -> Self {
        Self {
            nodes: vec![ExprNode::Any],
            root: 0,
        }
    }

    fn atom(value: T) -> Self {
        Self {
            nodes: vec![ExprNode::Atom(value)],
            root: 0,
        }
    }
}

impl<T: Clone> Expr<T> {
    fn eval(&self, f: &impl Fn(&T) -> bool) -> bool {
        self.eval_node(self.root, f)
    }

    fn eval_node(&self, index: usize, f: &impl Fn(&T) -> bool) -> bool {
        match &self.nodes[index] {
            ExprNode::Any => true,
            ExprNode::Atom(value) => f(value),
            ExprNode::Not(inner) => !self.eval_node(*inner, f),
            ExprNode::And(left, right) => self.eval_node(*left, f) && self.eval_node(*right, f),
            ExprNode::Or(left, right) => self.eval_node(*left, f) || self.eval_node(*right, f),
        }
    }

    fn combine(self, rhs: Self, node: impl FnOnce(usize, usize) -> ExprNode<T>) -> Self {
        let left_root = self.root;
        let offset = self.nodes.len();
        let right_root = offset + rhs.root;
        let mut nodes = self.nodes;
        nodes.extend(rhs.nodes.into_iter().map(|node| node.offset(offset)));
        nodes.push(node(left_root, right_root));
        let root = nodes.len() - 1;
        Self { nodes, root }
    }

    fn negate(self) -> Self {
        let root_index = self.root;
        let mut nodes = self.nodes;
        nodes.push(ExprNode::Not(root_index));
        let root = nodes.len() - 1;
        Self { nodes, root }
    }
}

#[derive(Debug, Clone)]
pub struct Predicate<T> {
    expr: Expr<T>,
}

impl<T> Predicate<T> {
    pub(crate) fn any() -> Self {
        Self { expr: Expr::any() }
    }
}

impl<T: Clone> BitAnd for Predicate<T> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            expr: self.expr.combine(rhs.expr, ExprNode::And),
        }
    }
}

impl<T: Clone> BitOr for Predicate<T> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            expr: self.expr.combine(rhs.expr, ExprNode::Or),
        }
    }
}

impl<T: Clone> Not for Predicate<T> {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self {
            expr: self.expr.negate(),
        }
    }
}

pub type ButtonPredicate = Predicate<MouseButton>;

impl Predicate<MouseButton> {
    pub(crate) fn button(button: MouseButton) -> Self {
        Self {
            expr: Expr::atom(button),
        }
    }

    pub(crate) fn matches(&self, pressed_buttons: &[MouseButton]) -> bool {
        self.expr
            .eval(&|button| pressed_buttons.iter().any(|pressed| pressed == button))
    }
}

impl From<MouseButton> for ButtonPredicate {
    fn from(button: MouseButton) -> Self {
        Self::button(button)
    }
}

impl BitAnd for MouseButton {
    type Output = ButtonPredicate;

    fn bitand(self, rhs: Self) -> Self::Output {
        ButtonPredicate::from(self) & ButtonPredicate::from(rhs)
    }
}

impl BitAnd<ButtonPredicate> for MouseButton {
    type Output = ButtonPredicate;

    fn bitand(self, rhs: ButtonPredicate) -> Self::Output {
        ButtonPredicate::from(self) & rhs
    }
}

impl BitAnd<MouseButton> for ButtonPredicate {
    type Output = Self;

    fn bitand(self, rhs: MouseButton) -> Self::Output {
        self & ButtonPredicate::from(rhs)
    }
}

impl BitOr for MouseButton {
    type Output = ButtonPredicate;

    fn bitor(self, rhs: Self) -> Self::Output {
        ButtonPredicate::from(self) | ButtonPredicate::from(rhs)
    }
}

impl BitOr<ButtonPredicate> for MouseButton {
    type Output = ButtonPredicate;

    fn bitor(self, rhs: ButtonPredicate) -> Self::Output {
        ButtonPredicate::from(self) | rhs
    }
}

impl BitOr<MouseButton> for ButtonPredicate {
    type Output = Self;

    fn bitor(self, rhs: MouseButton) -> Self::Output {
        self | ButtonPredicate::from(rhs)
    }
}

impl Not for MouseButton {
    type Output = ButtonPredicate;

    fn not(self) -> Self::Output {
        !ButtonPredicate::from(self)
    }
}

pub type KeyPredicate = Predicate<Key>;

impl Predicate<Key> {
    pub(crate) fn key(key: Key) -> Self {
        Self {
            expr: Expr::atom(key),
        }
    }

    pub(crate) fn matches(&self, key: &Key) -> bool {
        self.expr.eval(&|candidate| candidate == key)
    }
}

impl From<Key> for KeyPredicate {
    fn from(key: Key) -> Self {
        Self::key(key)
    }
}

impl From<NamedKey> for KeyPredicate {
    fn from(key: NamedKey) -> Self {
        Self::key(Key::from(key))
    }
}

impl BitAnd for Key {
    type Output = KeyPredicate;

    fn bitand(self, rhs: Self) -> Self::Output {
        KeyPredicate::from(self) & KeyPredicate::from(rhs)
    }
}

impl BitAnd<KeyPredicate> for Key {
    type Output = KeyPredicate;

    fn bitand(self, rhs: KeyPredicate) -> Self::Output {
        KeyPredicate::from(self) & rhs
    }
}

impl BitAnd<Key> for KeyPredicate {
    type Output = Self;

    fn bitand(self, rhs: Key) -> Self::Output {
        self & KeyPredicate::from(rhs)
    }
}

impl BitOr for Key {
    type Output = KeyPredicate;

    fn bitor(self, rhs: Self) -> Self::Output {
        KeyPredicate::from(self) | KeyPredicate::from(rhs)
    }
}

impl BitOr<KeyPredicate> for Key {
    type Output = KeyPredicate;

    fn bitor(self, rhs: KeyPredicate) -> Self::Output {
        KeyPredicate::from(self) | rhs
    }
}

impl BitOr<Key> for KeyPredicate {
    type Output = Self;

    fn bitor(self, rhs: Key) -> Self::Output {
        self | KeyPredicate::from(rhs)
    }
}

impl Not for Key {
    type Output = KeyPredicate;

    fn not(self) -> Self::Output {
        !KeyPredicate::from(self)
    }
}

impl BitAnd for NamedKey {
    type Output = KeyPredicate;

    fn bitand(self, rhs: Self) -> Self::Output {
        KeyPredicate::from(self) & KeyPredicate::from(rhs)
    }
}

impl BitAnd<KeyPredicate> for NamedKey {
    type Output = KeyPredicate;

    fn bitand(self, rhs: KeyPredicate) -> Self::Output {
        KeyPredicate::from(self) & rhs
    }
}

impl BitAnd<NamedKey> for KeyPredicate {
    type Output = Self;

    fn bitand(self, rhs: NamedKey) -> Self::Output {
        self & KeyPredicate::from(rhs)
    }
}

impl BitOr for NamedKey {
    type Output = KeyPredicate;

    fn bitor(self, rhs: Self) -> Self::Output {
        KeyPredicate::from(self) | KeyPredicate::from(rhs)
    }
}

impl BitOr<KeyPredicate> for NamedKey {
    type Output = KeyPredicate;

    fn bitor(self, rhs: KeyPredicate) -> Self::Output {
        KeyPredicate::from(self) | rhs
    }
}

impl BitOr<NamedKey> for KeyPredicate {
    type Output = Self;

    fn bitor(self, rhs: NamedKey) -> Self::Output {
        self | KeyPredicate::from(rhs)
    }
}

impl Not for NamedKey {
    type Output = KeyPredicate;

    fn not(self) -> Self::Output {
        !KeyPredicate::from(self)
    }
}

pub type ModifierPredicate = Predicate<Modifier>;

impl Predicate<Modifier> {
    pub(crate) fn modifier(modifier: Modifier) -> Self {
        Self {
            expr: Expr::atom(modifier),
        }
    }

    pub(crate) fn matches(&self, modifiers: Modifiers) -> bool {
        self.expr.eval(&|modifier| modifiers.contains(*modifier))
    }
}

impl From<Modifier> for ModifierPredicate {
    fn from(modifier: Modifier) -> Self {
        Self::modifier(modifier)
    }
}

impl BitAnd for Modifier {
    type Output = ModifierPredicate;

    fn bitand(self, rhs: Self) -> Self::Output {
        ModifierPredicate::from(self) & ModifierPredicate::from(rhs)
    }
}

impl BitAnd<ModifierPredicate> for Modifier {
    type Output = ModifierPredicate;

    fn bitand(self, rhs: ModifierPredicate) -> Self::Output {
        ModifierPredicate::from(self) & rhs
    }
}

impl BitAnd<Modifier> for ModifierPredicate {
    type Output = Self;

    fn bitand(self, rhs: Modifier) -> Self::Output {
        self & ModifierPredicate::from(rhs)
    }
}

impl BitOr for Modifier {
    type Output = ModifierPredicate;

    fn bitor(self, rhs: Self) -> Self::Output {
        ModifierPredicate::from(self) | ModifierPredicate::from(rhs)
    }
}

impl BitOr<ModifierPredicate> for Modifier {
    type Output = ModifierPredicate;

    fn bitor(self, rhs: ModifierPredicate) -> Self::Output {
        ModifierPredicate::from(self) | rhs
    }
}

impl BitOr<Modifier> for ModifierPredicate {
    type Output = Self;

    fn bitor(self, rhs: Modifier) -> Self::Output {
        self | ModifierPredicate::from(rhs)
    }
}

impl Not for Modifier {
    type Output = ModifierPredicate;

    fn not(self) -> Self::Output {
        !ModifierPredicate::from(self)
    }
}

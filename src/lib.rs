use std::{
    cmp::{Ordering, PartialOrd},
    fmt::Display,
};

use vec1::{vec1, Vec1};

use pretty::{BoxDoc, Doc};

pub trait Prefixable {
    fn is_prefix_of(&self, other: &Self) -> bool;
    fn prefix_ord(&self, other: &Self) -> Option<Ordering> {
        match (self.is_prefix_of(other), other.is_prefix_of(self)) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}

impl<T: PartialEq + PartialOrd> Prefixable for [T] {
    fn is_prefix_of(&self, other: &Self) -> bool {
        other.starts_with(self)
    }
}

impl Prefixable for &str {
    fn is_prefix_of(&self, other: &Self) -> bool {
        other.starts_with(self)
    }
}

impl<'a, T: Prefixable> Prefixable for &'a T {
    fn is_prefix_of(&self, other: &Self) -> bool {
        (*other).is_prefix_of(*self)
    }
}

#[macro_export]
macro_rules! tree {
    ($t:expr) => {{
        RoseTree::node($t)
    }};
    ( $t:expr => $( $x:tt )+ ) => {{
        RoseTree::new($t, vec![$($x)+])
    }};
}

#[derive(Debug, PartialEq, Eq)]
pub struct RoseTree<T> {
    pub value: T,
    pub children: Vec<RoseTree<T>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum IndentationError {
    EmptyIterator,
    IncoherentIndent,
    InvalidIndent,
    Internal,
}

impl<T> RoseTree<T> {
    pub fn to_doc(&self, vertical: bool) -> Doc<BoxDoc<()>>
    where
        T: Display,
    {
        let children = &self.children;
        if children.is_empty() {
            Doc::as_string(&self.value)
        } else {
            if vertical {
                let head = Doc::as_string(&self.value).append(Doc::newline());
                let child_docs = Doc::intersperse(
                    children.into_iter().map(|c| c.to_doc(vertical)),
                    Doc::newline(),
                )
                .append(Doc::newline());
                head.nest(2).append(child_docs.nest(2))
            } else {
                let head = Doc::as_string(&self.value)
                    .append(" ")
                    .append(Doc::text("=>"))
                    .append(Doc::newline());
                let child_docs = Doc::space().append(Doc::intersperse(
                    children.into_iter().map(|c| c.to_doc(vertical)),
                    ", ",
                ));
                head.append(child_docs.nest(2).group())
            }
        }
    }

    pub fn node(value: T) -> Self {
        RoseTree {
            value,
            children: vec![],
        }
    }

    pub fn new(value: T, children: Vec<RoseTree<T>>) -> Self {
        RoseTree { value, children }
    }

    pub fn from_prefixables<Pr: Prefixable>(
        mut iter: impl Iterator<Item = (Pr, T)>,
    ) -> Result<Vec<Self>, IndentationError> {
        let mut indented_forest_stack: Vec1<(Pr, Vec1<Self>)> = match iter.next() {
            Some((base_indent, first_value)) => {
                vec1![(base_indent, vec1![Self::node(first_value)])]
            }
            None => {
                return Ok(vec![]);
            }
        };

        for (current_indent, current_value) in iter {
            match current_indent.prefix_ord(&indented_forest_stack.last().0) {
                Some(Ordering::Equal) => {
                    indented_forest_stack
                        .last_mut()
                        .1
                        .push(Self::node(current_value));
                }
                Some(Ordering::Greater) => {
                    indented_forest_stack.push((current_indent, vec1![Self::node(current_value)]));
                }
                Some(Ordering::Less) => {
                    if Self::valid_indent(&current_indent, &indented_forest_stack) {
                        Self::prune_down_to(&current_indent, &mut indented_forest_stack);
                        indented_forest_stack
                            .last_mut()
                            .1
                            .push(Self::node(current_value));
                    } else {
                        return Err(IndentationError::InvalidIndent);
                    }
                }
                None => {
                    return Err(IndentationError::IncoherentIndent);
                }
            }
        }

        Self::prune_down(&mut indented_forest_stack);

        indented_forest_stack
            .into_vec()
            .pop()
            .map(|(_, t)| t.into_vec())
            .ok_or(IndentationError::Internal)
    }

    fn prune_down<Pr>(stack: &mut Vec1<(Pr, Vec1<Self>)>) {
        while let Ok((_, v)) = stack.try_pop() {
            let mut highest = v.into_vec();
            stack.last_mut().1.last_mut().children.append(&mut highest);
        }
    }

    fn valid_indent<Pr: Prefixable>(target_indent: &Pr, stack: &Vec1<(Pr, Vec1<Self>)>) -> bool {
        stack
            .into_iter()
            .any(|&(ref p, _)| p.prefix_ord(target_indent) == Some(Ordering::Equal))
    }

    fn prune_down_to<Pr: Prefixable>(target_indent: &Pr, stack: &mut Vec1<(Pr, Vec1<Self>)>) {
        while target_indent.prefix_ord(&stack.last().0) == Some(Ordering::Less) {
            let mut highest = stack.try_pop().unwrap().1.into_vec();
            stack.last_mut().1.last_mut().children.append(&mut highest);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn increasing_lines_trees() {
        let increasing_lines: Vec<(&str, i32)> = vec![(&"", 1), (&" ", 2), (&"  ", 3), (&"   ", 4)];

        assert_eq!(
            RoseTree::from_prefixables(increasing_lines.into_iter()),
            Ok(vec![tree![1 => tree![2 => tree![3 => tree![4]]]]])
        );
    }

    #[test]
    fn constant_indentation_trees() {
        let constant_lines: Vec<(&str, i32)> = vec![(&"", 1), (&"", 2), (&"", 3), (&"", 4)];

        assert_eq!(
            RoseTree::from_prefixables(constant_lines.into_iter()),
            Ok(vec![tree![1], tree![2], tree![3], tree![4],])
        );
    }

    #[test]
    fn incoherent_indentation() {
        let incoherent_lines: Vec<(&str, i32)> = vec![(&"", 1), (&" ", 2), (&"\t", 3)];

        assert_eq!(
            RoseTree::from_prefixables(incoherent_lines.into_iter()),
            Err(IndentationError::IncoherentIndent)
        );
    }

    #[test]
    fn base_indent_respected() {
        let off_base_lines: Vec<(&str, i32)> = vec![
            (&" ", 1),
            (&"  ", 2),
            (&" ", 3),
            (&"  ", 4),
            (&"  ", 5),
            (&" ", 6),
        ];

        assert_eq!(
            RoseTree::from_prefixables(off_base_lines.into_iter()),
            Ok(vec![
                tree![1 => tree![2],],
                tree![3 => tree![4], tree![5],],
                tree![6],
            ])
        );
    }

    #[test]
    fn base_indent_is_invalid_indent_error() {
        let wrong_base_lines: Vec<(&str, i32)> = vec![(&" ", 1), (&"", 2)];

        assert_eq!(
            RoseTree::from_prefixables(wrong_base_lines.into_iter()),
            Err(IndentationError::InvalidIndent)
        );
    }
}

// Copyright 2026 Oxide Computer Company

//! Cycles in the type graph, and the two passes that deal with them.

use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;

use crate::build::Type;
use crate::error::Error;

/// Walk the type graph depth-first, visiting each type at most once.
///
/// Each id in `roots` starts a traversal; a type reached from an earlier root
/// is not visited again. `visit` is called once per newly visited type with
/// that type's id and the ids of the types on the path from the root to it,
/// itself included. A child id in that path set is a back edge and therefore
/// closes a cycle. `visit` returns the children to descend into, or an error
/// that ends the walk.
///
/// The walk never looks at the types themselves: `visit` supplies the
/// children, so callers choose both the graph and what to do with the cycles
/// they find. [`break_cycles`] and [`check_anonymous_cycles`] each walk a
/// different edge set on top of this.
fn walk_type_graph<Id, E, F>(roots: Vec<Id>, mut visit: F) -> Result<(), E>
where
    Id: Clone + Ord,
    F: FnMut(&Id, &BTreeSet<Id>) -> Result<Vec<Id>, E>,
{
    enum Node<Id> {
        Start { type_id: Id },
        Processing { type_id: Id, children_ids: Vec<Id> },
    }

    let mut visited = BTreeSet::<Id>::new();

    for type_id in roots {
        if visited.contains(&type_id) {
            continue;
        }

        let mut active = BTreeSet::<Id>::new();
        let mut stack = Vec::<Node<Id>>::new();

        active.insert(type_id.clone());
        stack.push(Node::Start { type_id });

        while let Some(top) = stack.last_mut() {
            match top {
                // Skip right to the end since we've already seen this type.
                Node::Start { type_id } if visited.contains(type_id) => {
                    assert!(active.contains(type_id));

                    let type_id = type_id.clone();
                    *top = Node::Processing {
                        type_id,
                        children_ids: Vec::new(),
                    };
                }

                // Visit this type and queue up the children into which we
                // should descend.
                Node::Start { type_id } => {
                    assert!(active.contains(type_id));

                    let type_id = type_id.clone();
                    visited.insert(type_id.clone());

                    let children_ids = visit(&type_id, &active)?;

                    *top = Node::Processing {
                        type_id,
                        children_ids,
                    };
                }
                Node::Processing {
                    type_id,
                    children_ids: children,
                } => {
                    if let Some(child) = children.pop() {
                        active.insert(child.clone());
                        stack.push(Node::Start { type_id: child });
                    } else {
                        let type_id = type_id.clone();
                        active.remove(&type_id);
                        stack.pop();
                    }
                }
            }
        }
    }

    Ok(())
}

/// Break containment cycles so that no type is infinitely sized.
///
/// Walks the containment graph--[`Type::contained_children_mut`], the children
/// that contribute to a type's size--from every type. A child that closes a
/// cycle is replaced by a fresh [`Type::Box`] around it, which gives the cycle
/// a finite size; the remaining children are descended into.
pub(crate) fn break_cycles<Id, F>(types: &mut BTreeMap<Id, Type<Id>>, mut make_box_id: F)
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
    F: FnMut(&Id) -> Id,
{
    // A snapshot: the boxes minted below are never themselves roots.
    let roots = types.keys().cloned().collect();

    walk_type_graph(roots, |type_id, active| -> Result<Vec<Id>, Infallible> {
        // Determine which child types form cycles--and therefore need to be
        // snipped--and the rest--into which we should descend. We make this
        // its own block to clarify the lifetime of the exclusive reference to
        // the type. We don't really *need* to have an exclusive reference
        // here, but there's no point in writing `contained_children_mut`
        // again for shared references.
        let (snip, descend) = {
            let typ = types.get_mut(type_id).unwrap();

            let child_ids = typ
                .contained_children_mut()
                .into_iter()
                .map(|child_id| child_id.clone());

            // If the child type is in active then we've found a cycle
            // (otherwise we'll descend).
            child_ids.partition::<Vec<_>, _>(|child_id| active.contains(child_id))
        };

        // `snip` may contain duplicate ids, but `make_box_id` is
        // idempotent, so a duplicate maps to the same box id rather than
        // colliding with a different one under the same key.
        let replace = snip
            .into_iter()
            .map(|type_id| {
                let box_id = make_box_id(&type_id);
                let box_typ = Type::Box(type_id.clone());
                types.insert(box_id.clone(), box_typ);

                (type_id, box_id)
            })
            .collect::<BTreeMap<Id, Id>>();

        // Break any cycles by reassigning the child type to a box.
        let typ = types.get_mut(type_id).unwrap();

        let child_ids = typ.contained_children_mut();
        for child_id in child_ids {
            if let Some(replace_id) = replace.get(child_id) {
                *child_id = replace_id.clone();
            }
        }

        Ok(descend)
    })
    .unwrap()
}

/// Reject a cycle that passes through no named type.
///
/// A named type terminates the expansion rendering performs when it
/// code-generates the type: rendering `Vec<Inner>` only needs to write
/// `Inner`'s name, not expand it further. An anonymous (built-in or container)
/// type has no name to stop at, so a cycle made up entirely of anonymous types
/// has no finite representation--rendering it would recur forever.
/// [`break_cycles`] already ran, so this checks the post-boxing graph. Any
/// added Box types merely change the cardinality of a cycle.
///
/// Walks the subgraph induced on anonymous types: roots and children alike are
/// filtered to those for which [`Type::is_named`] is false. Reaching a named
/// child cuts that edge, since a name can never be part of an unrenderable
/// cycle. A back edge within that traversal is the error.
pub(crate) fn check_anonymous_cycles<Id>(types: &BTreeMap<Id, Type<Id>>) -> Result<(), Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    let roots = types
        .iter()
        .filter(|(_, typ)| !typ.is_named())
        .map(|(type_id, _)| type_id.clone())
        .collect();

    walk_type_graph(roots, |type_id, active| {
        let children_ids = types[type_id]
            .children()
            .into_iter()
            .filter(|child_id| !types[child_id].is_named())
            .collect::<Vec<_>>();

        // Any child in the active set indicates a cycle, and therefore
        // produces an error.
        match children_ids
            .iter()
            .find(|child_id| active.contains(*child_id))
        {
            Some(child_id) => Err(Error::AnonymousCycle {
                type_id: type_id.clone(),
                child_id: child_id.clone(),
            }),
            None => Ok(children_ids),
        }
    })
}

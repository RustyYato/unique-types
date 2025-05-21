use std::{
    borrow::Cow,
    sync::{Arc, Weak},
};

use unique_types::custom_counter;
use ut_cell::CellOwner;

custom_counter! {
    #[derive(Debug, Clone, Copy)]
    struct GraphCounter;
}

type Ut = unique_types::reusable_runtime::ReuseRuntimeUt<GraphCounter>;
type UtCell<T> = ut_cell::UtCell<T, Ut>;
const TOKEN: <Ut as unique_types::UniqueType>::Token = unique_types::TrivialToken::NEW;

pub struct Tree {
    owner: Ut,
    root: TreeNode,
}

#[derive(Clone)]
pub struct TreeNode {
    data: Arc<UtCell<TreeNodeData>>,
}

struct TreeNodeData {
    value: i64,
    parent: Weak<UtCell<TreeNodeData>>,
    children: Vec<TreeNode>,
}

impl Default for TreeNode {
    fn default() -> Self {
        Self::root()
    }
}

impl Tree {
    pub fn from_root(root: TreeNode) -> Self {
        Self {
            owner: Ut::with_counter(),
            root,
        }
    }

    pub fn new() -> Self {
        Self::from_root(TreeNode::root())
    }

    pub fn root(&self) -> &TreeNode {
        &self.root
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeNode {
    pub fn root() -> Self {
        let data = UtCell::from_token(
            TOKEN,
            TreeNodeData {
                value: 0,
                parent: Weak::new(),
                children: Vec::new(),
            },
        );

        Self {
            data: Arc::new(data),
        }
    }

    pub fn set_value(&self, x: i64, tree: &mut Tree) {
        tree.owner.get_mut(&self.data).value = x;
    }

    pub fn children<'a>(&'a self, tree: &'a Tree) -> &'a [Self] {
        &tree.owner.get(&self.data).children
    }

    #[track_caller]
    pub fn add_child<'a>(&self, child: impl Into<Cow<'a, Self>>, tree: &mut Tree) {
        self.add_child_(Cow::into_owned(child.into()), tree)
    }

    #[track_caller]
    fn add_child_(&self, child: Self, tree: &mut Tree) {
        let parent = &mut tree.owner.get_mut(&child.data).parent;
        assert!(
            parent.strong_count() == 0,
            "Cannot adopt another parent's child"
        );
        *parent = Arc::downgrade(&self.data);
        tree.owner.get_mut(&self.data).children.push(child);
    }

    pub fn parent(&self, tree: &Tree) -> Option<Self> {
        tree.owner
            .get(&self.data)
            .parent
            .upgrade()
            .map(|data| Self { data })
    }
}

impl From<TreeNode> for Cow<'_, TreeNode> {
    fn from(value: TreeNode) -> Self {
        Self::Owned(value)
    }
}

impl<'a, 'b: 'a> From<&'b TreeNode> for Cow<'a, TreeNode> {
    fn from(value: &'b TreeNode) -> Self {
        Self::Borrowed(value)
    }
}

#[allow(unused, clippy::extra_unused_type_parameters)]
fn test_send_sync<T: Send + Sync>() {
    let _ = test_send_sync::<TreeNode>;
}

#[test]
fn test1() {
    let mut tree = Tree::new();
    let root = tree.root().clone();
    assert!(root.parent(&tree).is_none());

    let left = TreeNode::root();
    root.add_child(&left, &mut tree);
    let right = TreeNode::root();
    root.add_child(right, &mut tree);

    assert_eq!(root.children(&tree).len(), 2);
}

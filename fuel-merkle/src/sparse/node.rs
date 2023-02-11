use crate::{
    common::{
        error::DeserializeError,
        path::{ComparablePath, Instruction, Path},
        Bytes32, ChildError, ChildResult, Node as NodeTrait, ParentNode as ParentNodeTrait, Prefix,
    },
    sparse::{
        hash::{sum, sum_all},
        zero_sum, Primitive,
    },
    storage::{Mappable, StorageInspect},
};

use crate::sparse::merkle_tree::MerkleTreeKey;
use core::marker::PhantomData;
use core::{cmp, fmt};

#[derive(Clone)]
pub(crate) struct Node<Key> {
    height: u32,
    prefix: Prefix,
    bytes_lo: Bytes32,
    bytes_hi: Bytes32,
    key: PhantomData<Key>,
}

impl<Key> Default for Node<Key> {
    fn default() -> Self {
        Self {
            height: Default::default(),
            prefix: Default::default(),
            bytes_lo: *zero_sum(),
            bytes_hi: *zero_sum(),
            key: Default::default(),
        }
    }
}

impl<Key> Node<Key>
where
    Key: MerkleTreeKey,
{
    pub fn max_height() -> usize {
        Node::<Key>::key_size_in_bits()
    }

    pub fn new(height: u32, prefix: Prefix, bytes_lo: Bytes32, bytes_hi: Bytes32) -> Self {
        Self {
            height,
            prefix,
            bytes_lo,
            bytes_hi,
            ..Default::default()
        }
    }

    pub fn create_leaf(key: &Bytes32, data: &[u8]) -> Self {
        Self {
            height: 0u32,
            prefix: Prefix::Leaf,
            bytes_lo: *key,
            bytes_hi: sum(data),
            ..Default::default()
        }
    }

    pub fn create_node(left_child: &Self, right_child: &Self, height: u32) -> Self {
        Self {
            height,
            prefix: Prefix::Node,
            bytes_lo: left_child.hash(),
            bytes_hi: right_child.hash(),
            ..Default::default()
        }
    }

    pub fn create_node_on_path(path: &dyn Path, path_node: &Self, side_node: &Self) -> Self {
        if path_node.is_leaf() && side_node.is_leaf() {
            // When joining two leaves, the joined node is found where the paths
            // of the two leaves diverge. The joined node may be a direct parent
            // of the leaves or an ancestor multiple generations above the
            // leaves.
            // N.B.: A leaf can be a placeholder.
            let parent_depth = path_node.common_path_length(side_node);
            let parent_height = (Self::max_height() - parent_depth) as u32;
            match path.get_instruction(parent_depth).unwrap() {
                Instruction::Left => Self::create_node(path_node, side_node, parent_height),
                Instruction::Right => Self::create_node(side_node, path_node, parent_height),
            }
        } else {
            // When joining two nodes, or a node and a leaf, the joined node is
            // the direct parent of the node with the greater height and an
            // ancestor of the node with the lesser height.
            // N.B.: A leaf can be a placeholder.
            let parent_height = cmp::max(path_node.height(), side_node.height()) + 1;
            let parent_depth = Self::max_height() - parent_height as usize;
            match path.get_instruction(parent_depth).unwrap() {
                Instruction::Left => Self::create_node(path_node, side_node, parent_height),
                Instruction::Right => Self::create_node(side_node, path_node, parent_height),
            }
        }
    }

    pub fn create_placeholder() -> Self {
        Default::default()
    }

    pub fn common_path_length(&self, other: &Self) -> usize {
        debug_assert!(self.is_leaf());
        debug_assert!(other.is_leaf());

        // If either of the nodes is a placeholder, the common path length is
        // defined to be 0. This is needed to prevent a 0 bit in the
        // placeholder's key from producing an erroneous match with a 0 bit in
        // the leaf's key.
        if self.is_placeholder() || other.is_placeholder() {
            0
        } else {
            self.leaf_key().common_path_length(&other.leaf_key())
        }
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn prefix(&self) -> Prefix {
        self.prefix
    }

    pub fn bytes_lo(&self) -> &Bytes32 {
        &self.bytes_lo
    }

    pub fn bytes_hi(&self) -> &Bytes32 {
        &self.bytes_hi
    }

    pub fn is_leaf(&self) -> bool {
        self.prefix() == Prefix::Leaf || self.is_placeholder()
    }

    pub fn is_node(&self) -> bool {
        self.prefix() == Prefix::Node
    }

    pub fn leaf_key(&self) -> Key {
        assert!(self.is_leaf());
        (*self.bytes_lo()).into()
    }

    pub fn leaf_data(&self) -> Key {
        assert!(self.is_leaf());
        (*self.bytes_hi()).into()
    }

    pub fn left_child_key(&self) -> Key {
        assert!(self.is_node());
        (*self.bytes_lo()).into()
    }

    pub fn right_child_key(&self) -> Key {
        assert!(self.is_node());
        (*self.bytes_hi()).into()
    }

    pub fn is_placeholder(&self) -> bool {
        *self.bytes_lo() == *zero_sum() && *self.bytes_hi() == *zero_sum()
    }

    pub fn hash(&self) -> Bytes32 {
        if self.is_placeholder() {
            *zero_sum()
        } else {
            let data = [self.prefix.as_ref(), self.bytes_lo.as_ref(), self.bytes_hi.as_ref()];
            sum_all(data)
        }
    }
}

impl<Key> AsRef<Node<Key>> for Node<Key> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<Key> NodeTrait for Node<Key>
where
    Key: MerkleTreeKey,
{
    type Key = Key;

    fn height(&self) -> u32 {
        Node::height(self)
    }

    fn leaf_key(&self) -> Self::Key {
        Node::leaf_key(self)
    }

    fn is_leaf(&self) -> bool {
        Node::is_leaf(self)
    }

    fn is_node(&self) -> bool {
        Node::is_node(self)
    }
}

impl<Key> fmt::Debug for Node<Key>
where
    Key: MerkleTreeKey,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_node() {
            f.debug_struct("Node (Internal)")
                .field("Height", &self.height())
                .field("Hash", &hex::encode(self.hash()))
                .field("Left child key", &hex::encode(self.left_child_key().as_ref()))
                .field("Right child key", &hex::encode(self.right_child_key().as_ref()))
                .finish()
        } else {
            f.debug_struct("Node (Leaf)")
                .field("Height", &self.height())
                .field("Hash", &hex::encode(self.hash()))
                .field("Leaf key", &hex::encode(self.leaf_key().as_ref()))
                .field("Leaf data", &hex::encode(self.leaf_data().as_ref()))
                .finish()
        }
    }
}

pub(crate) struct StorageNode<'storage, TableType, StorageType, Key> {
    storage: &'storage StorageType,
    node: Node<Key>,
    phantom_table: PhantomData<TableType>,
}

impl<TableType, StorageType, Key> Clone for StorageNode<'_, TableType, StorageType, Key>
where
    Key: Clone,
{
    fn clone(&self) -> Self {
        Self {
            storage: self.storage,
            node: self.node.clone(),
            phantom_table: Default::default(),
        }
    }
}

impl<'s, TableType, StorageType, Key> StorageNode<'s, TableType, StorageType, Key> {
    pub fn new(storage: &'s StorageType, node: Node<Key>) -> Self {
        Self {
            node,
            storage,
            phantom_table: Default::default(),
        }
    }
}

impl<TableType, StorageType, Key> StorageNode<'_, TableType, StorageType, Key>
where
    Key: MerkleTreeKey,
{
    pub fn hash(&self) -> Bytes32 {
        self.node.hash()
    }

    pub fn into_node(self) -> Node<Key> {
        self.node
    }
}

impl<TableType, StorageType, Key> NodeTrait for StorageNode<'_, TableType, StorageType, Key>
where
    Key: MerkleTreeKey,
{
    type Key = Key;

    fn height(&self) -> u32 {
        self.node.height()
    }

    fn leaf_key(&self) -> Self::Key {
        self.node.leaf_key()
    }

    fn is_leaf(&self) -> bool {
        self.node.is_leaf()
    }

    fn is_node(&self) -> bool {
        self.node.is_node()
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum StorageNodeError<StorageError> {
    #[cfg_attr(feature = "std", error(transparent))]
    StorageError(StorageError),
    #[cfg_attr(feature = "std", error(transparent))]
    DeserializeError(DeserializeError),
}

impl<TableType, StorageType, Key> ParentNodeTrait for StorageNode<'_, TableType, StorageType, Key>
where
    StorageType: StorageInspect<TableType>,
    TableType: Mappable<Key = Key, Value = Primitive, OwnedValue = Primitive>,
    Key: MerkleTreeKey + ComparablePath + PartialEq,
{
    type Error = StorageNodeError<StorageType::Error>;

    fn left_child(&self) -> ChildResult<Self> {
        if self.is_leaf() {
            return Err(ChildError::NodeIsLeaf);
        }
        let key = self.node.left_child_key();
        if key.as_ref() == zero_sum() {
            return Ok(Self::new(self.storage, Node::create_placeholder()));
        }
        let primitive = self
            .storage
            .get(&key.into())
            .map_err(StorageNodeError::StorageError)?
            .ok_or(ChildError::ChildNotFound(key))?;
        Ok(primitive
            .into_owned()
            .try_into()
            .map(|node| Self::new(self.storage, node))
            .map_err(StorageNodeError::DeserializeError)?)
    }

    fn right_child(&self) -> ChildResult<Self> {
        if self.is_leaf() {
            return Err(ChildError::NodeIsLeaf);
        }
        let key = self.node.right_child_key();
        if key.as_ref() == zero_sum() {
            return Ok(Self::new(self.storage, Node::create_placeholder()));
        }
        let primitive = self
            .storage
            .get(&key.into())
            .map_err(StorageNodeError::StorageError)?
            .ok_or(ChildError::ChildNotFound(key))?;
        Ok(primitive
            .into_owned()
            .try_into()
            .map(|node| Self::new(self.storage, node))
            .map_err(StorageNodeError::DeserializeError)?)
    }
}

impl<TableType, StorageType, Key> fmt::Debug for StorageNode<'_, TableType, StorageType, Key>
where
    StorageType: StorageInspect<TableType>,
    TableType: Mappable<Key = Key, Value = Primitive, OwnedValue = Primitive>,
    Key: MerkleTreeKey + ComparablePath,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_node() {
            f.debug_struct("StorageNode (Internal)")
                .field("Height", &self.height())
                .field("Hash", &hex::encode(self.hash()))
                .field("Left child key", &hex::encode(self.node.left_child_key().as_ref()))
                .field("Right child key", &hex::encode(self.node.right_child_key().as_ref()))
                .finish()
        } else {
            f.debug_struct("StorageNode (Leaf)")
                .field("Height", &self.height())
                .field("Hash", &hex::encode(self.hash()))
                .field("Leaf key", &hex::encode(self.node.leaf_key().as_ref()))
                .field("Leaf data", &hex::encode(self.node.leaf_data().as_ref()))
                .finish()
        }
    }
}

#[cfg(test)]
mod test_node {
    use crate::common::WrappedBytes32;
    use crate::{
        common::{error::DeserializeError, Bytes32, Prefix, PrefixError},
        sparse::{
            hash::{sum, sum_all},
            zero_sum, Node, Primitive,
        },
    };

    fn leaf_hash<V>(key: &Bytes32, data: &[u8]) -> V
    where
        V: From<Bytes32>,
    {
        let s = sum::<_, Bytes32>(data);
        let data = [Prefix::Leaf.as_ref(), key.as_slice(), s.as_slice()];
        sum_all(data)
    }

    fn node_hash<V>(left: &Bytes32, right: &Bytes32) -> V
    where
        V: From<Bytes32>,
    {
        let data = [Prefix::Node.as_ref(), left.as_slice(), right.as_slice()];
        sum_all(data)
    }

    #[test]
    fn test_create_leaf_returns_a_valid_leaf() {
        let leaf = Node::<WrappedBytes32>::create_leaf(&sum(b"LEAF"), &[1u8; 32]);
        assert_eq!(leaf.is_leaf(), true);
        assert_eq!(leaf.is_node(), false);
        assert_eq!(leaf.height(), 0);
        assert_eq!(leaf.prefix(), Prefix::Leaf);
        assert_eq!(leaf.leaf_key(), sum::<_, WrappedBytes32>(b"LEAF"));
        assert_eq!(leaf.leaf_data(), sum::<_, WrappedBytes32>([1u8; 32]));
    }

    #[test]
    fn test_create_node_returns_a_valid_node() {
        let left_child = Node::<WrappedBytes32>::create_leaf(&sum(b"LEFT CHILD"), &[1u8; 32]);
        let right_child = Node::<WrappedBytes32>::create_leaf(&sum(b"RIGHT CHILD"), &[1u8; 32]);
        let node = Node::<WrappedBytes32>::create_node(&left_child, &right_child, 1);
        assert_eq!(node.is_leaf(), false);
        assert_eq!(node.is_node(), true);
        assert_eq!(node.height(), 1);
        assert_eq!(node.prefix(), Prefix::Node);
        assert_eq!(node.left_child_key(), leaf_hash(&sum(b"LEFT CHILD"), &[1u8; 32]));
        assert_eq!(node.right_child_key(), leaf_hash(&sum(b"RIGHT CHILD"), &[1u8; 32]));
    }

    #[test]
    fn test_create_placeholder_returns_a_placeholder_node() {
        let node = Node::<WrappedBytes32>::create_placeholder();
        assert_eq!(node.is_placeholder(), true);
        assert_eq!(node.hash(), *zero_sum());
    }

    #[test]
    fn test_create_leaf_from_primitive_returns_a_valid_leaf() {
        let primitive = (0, Prefix::Leaf as u8, [0xff; 32], [0xff; 32]);

        let node: Node<WrappedBytes32> = primitive.try_into().unwrap();
        assert_eq!(node.is_leaf(), true);
        assert_eq!(node.is_node(), false);
        assert_eq!(node.height(), 0);
        assert_eq!(node.prefix(), Prefix::Leaf);
        assert_eq!(node.leaf_key(), [0xff; 32].into());
        assert_eq!(node.leaf_data(), [0xff; 32].into());
    }

    #[test]
    fn test_create_node_from_primitive_returns_a_valid_node() {
        let primitive = (255, Prefix::Node as u8, [0xff; 32], [0xff; 32]);

        let node: Node<WrappedBytes32> = primitive.try_into().unwrap();
        assert_eq!(node.is_leaf(), false);
        assert_eq!(node.is_node(), true);
        assert_eq!(node.height(), 255);
        assert_eq!(node.prefix(), Prefix::Node);
        assert_eq!(node.left_child_key(), [0xff; 32].into());
        assert_eq!(node.right_child_key(), [0xff; 32].into());
    }

    #[test]
    fn test_create_from_primitive_returns_deserialize_error_if_invalid_prefix() {
        let primitive = (0xff, 0xff, [0xff; 32], [0xff; 32]);

        // Should return Error; prefix 0xff is does not represent a node or leaf
        let err = Node::<WrappedBytes32>::try_from(primitive).expect_err("Expected try_from() to be Error; got OK");
        assert!(matches!(
            err,
            DeserializeError::PrefixError(PrefixError::InvalidPrefix(0xff))
        ));
    }

    /// For leaf node `node` of leaf data `d` with key `k`:
    /// ```node = (0x00, k, h(serialize(d)))```
    #[test]
    fn test_leaf_primitive_returns_expected_primitive() {
        let expected_primitive = (0_u32, Prefix::Leaf as u8, sum(b"LEAF"), sum([1u8; 32]));

        let leaf = Node::<WrappedBytes32>::create_leaf(&sum(b"LEAF"), &[1u8; 32]);
        let primitive = Primitive::from(&leaf);

        assert_eq!(primitive, expected_primitive);
    }

    /// For internal node `node` with children `l` and `r`:
    /// ```node = (0x01, l.v, r.v)```
    #[test]
    fn test_node_primitive_returns_expected_primitive() {
        let expected_primitive = (
            1_u32,
            Prefix::Node as u8,
            leaf_hash(&sum(b"LEFT CHILD"), &[1u8; 32]),
            leaf_hash(&sum(b"RIGHT CHILD"), &[1u8; 32]),
        );

        let left_child = Node::<WrappedBytes32>::create_leaf(&sum(b"LEFT CHILD"), &[1u8; 32]);
        let right_child = Node::<WrappedBytes32>::create_leaf(&sum(b"RIGHT CHILD"), &[1u8; 32]);
        let node = Node::create_node(&left_child, &right_child, 1);
        let primitive = Primitive::from(&node);

        assert_eq!(primitive, expected_primitive);
    }

    /// For leaf node `node` of leaf data `d` with key `k`:
    /// ```node.v = h(0x00, k, h(serialize(d)))```
    #[test]
    fn test_leaf_hash_returns_expected_hash_value() {
        let expected_value: Bytes32 = leaf_hash(&sum(b"LEAF"), &[1u8; 32]);

        let node = Node::<WrappedBytes32>::create_leaf(&sum(b"LEAF"), &[1u8; 32]);
        let value = node.hash();

        assert_eq!(value, expected_value);
    }

    /// For internal node `node` with children `l` and `r`:
    /// ```node.v = h(0x01, l.v, r.v)```
    #[test]
    fn test_node_hash_returns_expected_hash_value() {
        let left = leaf_hash(&sum(b"LEFT CHILD"), &[1u8; 32]);
        let right = leaf_hash(&sum(b"RIGHT CHILD"), &[1u8; 32]);
        let expected_value: Bytes32 = node_hash(&left, &right);

        let left_child = Node::<WrappedBytes32>::create_leaf(&sum(b"LEFT CHILD"), &[1u8; 32]);
        let right_child = Node::<WrappedBytes32>::create_leaf(&sum(b"RIGHT CHILD"), &[1u8; 32]);
        let node = Node::create_node(&left_child, &right_child, 1);
        let value = node.hash();

        assert_eq!(value, expected_value);
    }
}

#[cfg(test)]
mod test_storage_node {
    use crate::{
        common::{error::DeserializeError, ChildError, ParentNode, PrefixError, StorageMap, WrappedBytes32},
        sparse::{hash::sum, node::StorageNodeError, Node, Primitive, StorageNode},
        storage::{Mappable, StorageMutate},
    };

    struct TestTable;
    impl Mappable for TestTable {
        type Key = Self::OwnedKey;
        type OwnedKey = WrappedBytes32;
        type Value = Self::OwnedValue;
        type OwnedValue = Primitive;
    }

    #[test]
    fn test_node_left_child_returns_the_left_child() {
        let mut s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), &[1u8; 32]);
        let _ = s.insert(&leaf_0.hash().into(), &leaf_0.as_ref().into());

        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let _ = s.insert(&leaf_1.hash().into(), &leaf_1.as_ref().into());

        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);
        let _ = s.insert(&node_0.hash().into(), &node_0.as_ref().into());

        let storage_node = StorageNode::new(&s, node_0);
        let child = storage_node.left_child().unwrap();

        assert_eq!(child.hash(), leaf_0.hash());
    }

    #[test]
    fn test_node_right_child_returns_the_right_child() {
        let mut s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), &[1u8; 32]);
        let _ = s.insert(&leaf_0.hash().into(), &leaf_0.as_ref().into());

        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let _ = s.insert(&leaf_1.hash().into(), &leaf_1.as_ref().into());

        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);
        let _ = s.insert(&node_0.hash().into(), &node_0.as_ref().into());

        let storage_node = StorageNode::new(&s, node_0);
        let child = storage_node.right_child().unwrap();

        assert_eq!(child.hash(), leaf_1.hash());
    }

    #[test]
    fn test_node_left_child_returns_placeholder_when_key_is_zero_sum() {
        let mut s = StorageMap::<TestTable>::new();

        let leaf = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let _ = s.insert(&leaf.hash().into(), &leaf.as_ref().into());

        let node_0 = Node::create_node(&Node::create_placeholder(), &leaf, 1);
        let _ = s.insert(&node_0.hash().into(), &node_0.as_ref().into());

        let storage_node = StorageNode::new(&s, node_0);
        let child = storage_node.left_child().unwrap();

        assert!(child.node.is_placeholder());
    }

    #[test]
    fn test_node_right_child_returns_placeholder_when_key_is_zero_sum() {
        let mut s = StorageMap::<TestTable>::new();

        let leaf = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let _ = s.insert(&leaf.hash().into(), &leaf.as_ref().into());

        let node_0 = Node::create_node(&leaf, &Node::create_placeholder(), 1);
        let _ = s.insert(&node_0.hash().into(), &node_0.as_ref().into());

        let storage_node = StorageNode::new(&s, node_0);
        let child = storage_node.right_child().unwrap();

        assert!(child.node.is_placeholder());
    }

    #[test]
    fn test_node_left_child_returns_error_when_node_is_leaf() {
        let s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), &[1u8; 32]);
        let storage_node = StorageNode::new(&s, leaf_0);
        let err = storage_node
            .left_child()
            .expect_err("Expected left_child() to return Error; got OK");

        assert!(matches!(err, ChildError::NodeIsLeaf));
    }

    #[test]
    fn test_node_right_child_returns_error_when_node_is_leaf() {
        let s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), &[1u8; 32]);
        let storage_node = StorageNode::new(&s, leaf_0);
        let err = storage_node
            .right_child()
            .expect_err("Expected right_child() to return Error; got OK");

        assert!(matches!(err, ChildError::NodeIsLeaf));
    }

    #[test]
    fn test_node_left_child_returns_error_when_key_is_not_found() {
        let s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), &[0u8; 32]);
        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);

        let storage_node = StorageNode::new(&s, node_0);
        let err = storage_node
            .left_child()
            .expect_err("Expected left_child() to return Error; got Ok");

        let key = storage_node.into_node().left_child_key();
        assert!(matches!(
            err,
            ChildError::ChildNotFound(k) if k == key
        ));
    }

    #[test]
    fn test_node_right_child_returns_error_when_key_is_not_found() {
        let s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), &[1u8; 32]);
        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);

        let storage_node = StorageNode::new(&s, node_0);
        let err = storage_node
            .right_child()
            .expect_err("Expected right_child() to return Error; got Ok");

        let key = storage_node.into_node().right_child_key();
        assert!(matches!(
            err,
            ChildError::ChildNotFound(k) if k == key
        ));
    }

    #[test]
    fn test_node_left_child_returns_deserialize_error_when_primitive_is_invalid() {
        let mut s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), &[1u8; 32]);
        let _ = s.insert(&leaf_0.hash().into(), &(0xff, 0xff, [0xff; 32], [0xff; 32]));
        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);

        let storage_node = StorageNode::new(&s, node_0);
        let err = storage_node
            .left_child()
            .expect_err("Expected left_child() to be Error; got Ok");

        assert!(matches!(
            err,
            ChildError::Error(StorageNodeError::DeserializeError(DeserializeError::PrefixError(
                PrefixError::InvalidPrefix(0xff)
            )))
        ));
    }

    #[test]
    fn test_node_right_child_returns_deserialize_error_when_primitive_is_invalid() {
        let mut s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), &[1u8; 32]);
        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let _ = s.insert(&leaf_1.hash().into(), &(0xff, 0xff, [0xff; 32], [0xff; 32]));
        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);

        let storage_node = StorageNode::new(&s, node_0);
        let err = storage_node
            .right_child()
            .expect_err("Expected right_child() to be Error; got Ok");

        assert!(matches!(
            err,
            ChildError::Error(StorageNodeError::DeserializeError(DeserializeError::PrefixError(
                PrefixError::InvalidPrefix(0xff)
            )))
        ));
    }
}
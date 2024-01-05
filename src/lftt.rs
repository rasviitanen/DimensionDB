use crate::ebr::Owned;

pub enum TxStatus {
    Active,
    Commited,
    Aborted,
}

#[derive(Clone, Copy)]
pub enum OpType {
    Insert,
    Delete,
    InsertEdge,
    DeleteEdge,
    Find,
}

pub struct Operation<const DIM: usize> {
    pub ty: OpType,
    pub key: [u8; DIM],
}

pub struct Desc<const DIM: usize> {
    pub size: usize,
    pub status: TxStatus,
    pub current_op: usize,
    pub ops: Vec<Operation<DIM>>,
}

pub struct NodeDesc<const DIM: usize> {
    pub desc: Owned<Desc<DIM>>,
    pub opid: usize,
    pub override_as_find: bool,
    pub override_as_delete: bool,
}

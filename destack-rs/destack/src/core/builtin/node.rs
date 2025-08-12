use uuid::Uuid;

use crate::core::builtin::object::Object;

// nocheckin: generate NodeComponents (Object?Component)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeReference {
    pub r#type: i8,
    pub id: Uuid,
    pub space_id: Uuid,
    pub branch_id: Uuid,
    pub snapshot_id: Uuid,
}

pub trait Node: Object {
    #[inline]
    fn metakind(&self) -> u8 {
        2
    }

    fn metatype(&self) -> u8;

    fn id(&self) -> Uuid;
    fn space_id(&self) -> Uuid;
    fn branch_id(&self) -> Uuid;
    fn snapshot_id(&self) -> Uuid;

    // fn space(&self, session: &Session) -> &Space;
}

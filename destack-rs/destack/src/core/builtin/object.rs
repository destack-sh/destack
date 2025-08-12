pub trait Object {
    fn metakind(&self) -> u8;

    fn metatype(&self) -> u8;
}

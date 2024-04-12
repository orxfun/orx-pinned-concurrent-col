/// Defines the action that the pinned concurrent collection must take on a write request to a given position, or to a given range of positions.
#[derive(Debug)]
pub enum WritePermit {
    /// Concurrent collection is allowed to directly write to the positions without any delay.
    JustWrite,
    /// Concurrent collection needs to grow to perform the write request.
    /// Furthermore, the caller thread must take the responsibility of the allocation.
    /// Then, it is free to write the value or values.
    GrowThenWrite,
    /// The caller thread must spin and re-evaluate the write permit.
    Spin,
}

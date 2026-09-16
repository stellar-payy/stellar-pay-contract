use soroban_sdk::contracterror;

// === Payment Registry Errors

// Every failure mode is a catchable Result variant, never a bare panic,
// so the backend caller can distinguish a genuine business rejection
// (for example a retried, already-recorded payment) from a host error.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PaymentRegistryError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    InvalidAmount = 4,
    AlreadyRecorded = 5,
    NotFound = 6,
}

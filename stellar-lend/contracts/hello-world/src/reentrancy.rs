use soroban_sdk::{Env, Symbol, IntoVal, Val};

pub struct ReentrancyGuard<'a> {
    env: &'a Env,
    key: Val,
}

impl<'a> ReentrancyGuard<'a> {
    /// Create a new global reentrancy guard.
    pub fn new(env: &'a Env) -> Result<Self, u32> {
        let key = Symbol::new(env, "REENTRANCY_LOCK").into_val(env);
        Self::new_with_key(env, key)
    }

    /// Create a new reentrancy guard with a specific key.
    pub fn new_with_key(env: &'a Env, key: Val) -> Result<Self, u32> {
        if env.storage().temporary().has(&key) {
            // Error code 7 corresponds to Reentrancy in all operation error enums
            return Err(7);
        }
        env.storage().temporary().set(&key, &true);
        Ok(Self { env, key })
    }
}

impl<'a> Drop for ReentrancyGuard<'a> {
    fn drop(&mut self) {
        self.env.storage().temporary().remove(&self.key);
    }
}

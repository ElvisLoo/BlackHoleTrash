// Elevated startup guard: removed in ElvisLoo fork to allow admin users.
// Upstream blocked admin startup because Windows UIPI prevents normal-privilege
// Explorer drag-drop to an elevated window. Fork users accept this limitation.

pub fn allow_startup() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fork_always_allows_startup() {
        assert!(allow_startup());
    }
}
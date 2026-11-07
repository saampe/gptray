//! Dispatch/registration policy independent of the operating system registry.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    ToggleQuick,
    OpenFull,
}
pub fn action(id: u32, pressed: bool, quick: u32, full: u32) -> Option<Command> {
    if !pressed {
        None
    } else if id == quick {
        Some(Command::ToggleQuick)
    } else if id == full {
        Some(Command::OpenFull)
    } else {
        None
    }
}
pub fn register_both<T, E>(
    quick: T,
    full: T,
    mut register: impl FnMut(T) -> Result<(), E>,
) -> [Result<(), E>; 2] {
    [register(quick), register(full)]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pressed_shortcuts_dispatch_independent_actions() {
        assert_eq!(action(7, true, 7, 8), Some(Command::ToggleQuick));
        assert_eq!(action(8, true, 7, 8), Some(Command::OpenFull));
        assert_eq!(action(7, false, 7, 8), None);
        assert_eq!(action(8, false, 7, 8), None);
        assert_eq!(action(9, true, 7, 8), None);
    }
    #[test]
    fn either_registration_conflict_does_not_skip_the_other() {
        for blocked in [7, 8] {
            let mut attempts = Vec::new();
            let results = register_both(7, 8, |id| {
                attempts.push(id);
                if id == blocked {
                    Err("conflict")
                } else {
                    Ok(())
                }
            });
            assert_eq!(attempts, [7, 8]);
            assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        }
    }
}

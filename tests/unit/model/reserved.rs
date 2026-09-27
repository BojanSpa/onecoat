use super::{OMP_BUILTIN_THEMES, is_omp_builtin};

#[test]
fn reserved_names_cover_every_builtin() {
    assert_eq!(OMP_BUILTIN_THEMES.len(), 101);
    for name in ["titanium", "dark", "dark-nord", "light-zenith"] {
        assert!(is_omp_builtin(name), "{name} must be reserved");
    }
    for name in ["nord", "onecoat-dark"] {
        assert!(!is_omp_builtin(name), "{name} must stay free");
    }
}

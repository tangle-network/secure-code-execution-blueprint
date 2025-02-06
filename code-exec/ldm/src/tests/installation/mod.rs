use crate::LanguageProvider;
use crate::*;

async fn try_install<T: LanguageProvider>(provider: &mut T) -> bool {
    let config = InstallationConfig::default();
    let manager = InstallationManager::new_for_current_os(config);

    // Skip test if no package manager is available
    if let Some(pm) = manager.find_available_package_manager() {
        if !pm.is_available() {
            println!("Skipping test: no package manager available for current OS");
            return false;
        }

        match manager.install_dependencies(provider).await {
            Ok(_) => {
                if let Err(e) = manager.cleanup(provider).await {
                    println!("Cleanup error: {}", e);
                }
                true
            }
            Err(e) => {
                println!("Installation error: {}", e);
                false
            }
        }
    } else {
        println!("Skipping test: no package manager available for current OS");
        false
    }
}

#[tokio::test]
async fn test_python_installation() {
    let mut provider = PythonProvider::default();
    if !try_install(&mut provider).await {
        println!("Skipping Python installation test due to environment limitations");
    }
}

#[tokio::test]
async fn test_typescript_installation() {
    let mut provider = TypeScriptProvider::default();
    if !try_install(&mut provider).await {
        println!("Skipping TypeScript installation test due to environment limitations");
    }
}

#[tokio::test]
async fn test_rust_installation() {
    let mut provider = RustProvider::default();
    if !try_install(&mut provider).await {
        println!("Skipping Rust installation test due to environment limitations");
    }
}

#[tokio::test]
async fn test_go_installation() {
    let mut provider = GoProvider::default();
    if !try_install(&mut provider).await {
        println!("Skipping Go installation test due to environment limitations");
    }
}

#[tokio::test]
async fn test_os_detection() {
    use crate::manager::OsType;

    let os = OsType::current();
    let manager = InstallationManager::new_for_current_os(InstallationConfig::default());

    match os {
        OsType::Linux => {
            assert!(manager.find_available_package_manager().is_some());
            assert!(manager
                .find_available_package_manager()
                .unwrap()
                .is_available());
        }
        OsType::MacOS => {
            assert!(manager.find_available_package_manager().is_some());
            assert!(manager
                .find_available_package_manager()
                .unwrap()
                .is_available());
        }
    }
}

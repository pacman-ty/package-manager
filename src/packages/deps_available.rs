use crate::packages::Dependency;
use crate::Packages;
use rpkg::debversion;

impl Packages {
    /// Gets the dependencies of package_name, and prints out whether they are satisfied (and by which library/version) or not.
    pub fn deps_available(&self, package_name: &str) {
        if !self.package_exists(package_name) {
            println!("no such package {}", package_name);
            return;
        }

        println!("Package {}:", package_name);

        let package_num = self.get_package_num(package_name);
        let deps = self.dependencies.get(package_num).unwrap();

        for dep in deps {
            // Print the dependency string
            println!("- dependency {:?}", self.dep2str(dep));

            // Check if this dependency is satisfied
            match self.dep_is_satisfied(dep) {
                Some(pkg_name) => {
                    let installed_ver = self.get_installed_debver(pkg_name).unwrap();
                    println!("+ {} satisfied by installed version {}", pkg_name, installed_ver);
                },
                None => println!("-> not satisfied"),
            }
        }

    }

    /// Returns Some(package) which satisfies dependency dd, or None if not satisfied.
    pub fn dep_is_satisfied(&self, dd: &Dependency) -> Option<&str> {
        // Loop through each alternative in the dependency
        for alternative in dd {
            let pkg_num = alternative.package_num;
            let pkg_name = self.get_package_name(pkg_num);
            
            // Check if the package is installed
            match self.get_installed_debver(pkg_name) {
                None => continue,
                Some(installed_version) => {
                    match &alternative.rel_version {
                        // no version constraints 
                        None => return Some(pkg_name),
                        Some((op, ver_str)) => {
                            let required_version = ver_str.parse::<debversion::DebianVersionNum>().unwrap();
                            if debversion::cmp_debversion_with_op(op, installed_version, &required_version) {
                                return Some(pkg_name);
                            }
                            // Version doesn't match, try next alternative
                        }
                    }
                }
            }
        }
        // None of the alternatives satisfied the dependency
        None
    }

 
    /// Returns a Vec of packages which would satisfy dependency dd but for the version.
    /// Used by the how-to-install command, which calls compute_how_to_install().
    pub fn dep_satisfied_by_wrong_version(&self, dd: &Dependency) -> Vec<&str> {
        assert!(self.dep_is_satisfied(dd).is_none());
        let mut result = vec![];
        // Loop through each alternative in the dependency
        for alternative in dd {
            let pkg_num = alternative.package_num;
            let pkg_name = self.get_package_name(pkg_num);

            // Check if the package is installed
            if let Some(installed_version) = self.get_installed_debver(pkg_name) {
                match &alternative.rel_version {
                    None => {
                        // No version constraint, this shouldn't happen since dep_is_satisfied would return Some
                        // But if it does, skip it
                    },
                    Some((op, ver_str)) => {
                        let required_version = ver_str.parse::<debversion::DebianVersionNum>().unwrap();
                        if !debversion::cmp_debversion_with_op(op, installed_version, &required_version) {
                            // Wrong version installed!
                            result.push(pkg_name);
                        }
                    }
                }
            }
        }
        result
    }
}

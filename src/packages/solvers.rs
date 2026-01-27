use crate::Packages;
use crate::packages::Dependency;
use rpkg::debversion;

impl Packages {
    /// Computes a solution for the transitive dependencies of package_name; when there is a choice A | B | C, 
    /// chooses the first option A. Returns a Vec<i32> of package numbers.
    ///
    /// Note: does not consider which packages are installed.
    pub fn transitive_dep_solution(&self, package_name: &str) -> Vec<i32> {
        if !self.package_exists(package_name) {
            return vec![];
        }

        let deps: &Vec<Dependency> = &*self
            .dependencies
            .get(self.get_package_num(package_name))
            .unwrap();
        let mut dependency_set = vec![];

        // Populate dependency_set with first alternatives of package_name
        for dep in deps {
            if let Some(first_alternative) = dep.first() {
                dependency_set.push(first_alternative.package_num);
            }
        }

        // Iterate through the dependency set and add new dependencies
        loop {
            let mut new_deps_added = false;
            let current_size = dependency_set.len();
            // Iterate through packages already in dependency_set
            for i in 0..current_size {
                let pkg_num = dependency_set[i];
                // Get dependencies of this package
                if let Some(pkg_deps) = self.dependencies.get(&pkg_num) {
                    // Add first alternative of each dependency
                    for dep in pkg_deps {
                        if let Some(first_alternative) = dep.first() {
                            let dep_pkg_num = first_alternative.package_num;
                            // Only add if not already in dependency_set
                            if !dependency_set.contains(&dep_pkg_num) {
                                dependency_set.push(dep_pkg_num);
                                new_deps_added = true;
                            }
                        }
                    }
                }
            }
            // Stop if an iteration didn't add any new dependencies
            if !new_deps_added {
                break;
            }
        }
        return dependency_set;
    }
    
    /// Computes a set of packages that need to be installed to satisfy package_name's deps given the current installed packages.
    /// When a dependency A | B | C is unsatisfied, there are two possible cases:
    ///   (1) there are no versions of A, B, or C installed; pick the alternative with the highest version number (yes, compare apples and oranges).
    ///   (2) at least one of A, B, or C is installed (say A, B), but with the wrong version; of the installed packages (A, B), pick the one with the highest version number.
    pub fn compute_how_to_install(&self, package_name: &str) -> Vec<i32> {
        if !self.package_exists(package_name) {
            return vec![];
        }

        let mut dependencies_to_add: Vec<i32> = vec![];
        let package_num = *self.get_package_num(package_name);
        let deps = self.dependencies.get(&package_num).unwrap();

        // Populate dependencies_to_add with packages that need to be installed
        for dep in deps {
            if self.dep_is_satisfied(dep).is_some() { continue; } 

            let pkg_to_install = self.pick_alternative_to_install(dep);

            if let Some(pkg_num) = pkg_to_install {
                if !dependencies_to_add.contains(&pkg_num) {
                    dependencies_to_add.push(pkg_num);
                }
            }
        }

        loop {
            let mut new_deps_added = false;
            let current_size = dependencies_to_add.len();
           
            // Iterate through packages in dependencies_to_add
            for i in 0..current_size {
                let pkg_num = dependencies_to_add[i];

                if let Some(pkg_deps) = self.dependencies.get(&pkg_num) {
                    for dep in pkg_deps {
                        if self.dep_is_satisfied(dep).is_some() {
                            continue; // Already satisfied, skip

                        }

                        let dep_pkg_to_install = self.pick_alternative_to_install(dep);

                        if let Some(dep_pkg_num) = dep_pkg_to_install {
                            if !dependencies_to_add.contains(&dep_pkg_num) {
                                dependencies_to_add.push(dep_pkg_num);
                                new_deps_added = true;
                            }
                        }
                    }
                }
            }

            if !new_deps_added {
                break;
            }
        }
        return dependencies_to_add;
    }

 

    /// Helper function: picks which alternative to install when a dependency is unsatisfied.
    /// Returns the package number to install, or None if no valid alternative exists.
    fn pick_alternative_to_install(&self, dep: &Dependency) -> Option<i32> {
        let wrong_version_pkgs = self.dep_satisfied_by_wrong_version(dep);

        if !wrong_version_pkgs.is_empty() {

            let mut best_pkg_num: Option<i32> = None;
            let mut best_version: Option<&debversion::DebianVersionNum> = None;

            for &pkg_name in &wrong_version_pkgs {
                let pkg_num = *self.get_package_num(pkg_name);

                if let Some(available_ver) = self.get_available_debver(pkg_name) {
                    match best_version {
                        None => {
                            best_version = Some(available_ver);
                            best_pkg_num = Some(pkg_num);
                        }
                        Some(bv) if available_ver > bv => {
                            best_version = Some(available_ver);
                            best_pkg_num = Some(pkg_num);
                        }
                        _ => {}
                    }
                }
            }

            return best_pkg_num;
        } else {
            let mut best_pkg_num: Option<i32> = None;
            let mut best_version: Option<&debversion::DebianVersionNum> = None;

            for alternative in dep {
                let pkg_num = alternative.package_num;
                let pkg_name = self.get_package_name(pkg_num);

                if let Some(available_ver) = self.get_available_debver(pkg_name) {
                    match best_version {
                        None => {
                            best_version = Some(available_ver);
                            best_pkg_num = Some(pkg_num);
                        }
                        Some(bv) if available_ver > bv => {
                            best_version = Some(available_ver);
                            best_pkg_num = Some(pkg_num);
                        }
                        _ => {}
                    }
                }
            }
            return best_pkg_num;
        }
    }
}

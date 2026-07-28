let project_name = variable::get("project-name");
let github_owner = variable::get("github_owner");

if !variable::is_set("homebrew_formula") {
    variable::set("homebrew_formula", project_name);
}

if !variable::is_set("homebrew_tap") {
    variable::set("homebrew_tap", `${github_owner}/homebrew-brew`);
}

if !variable::is_set("repository") {
    variable::set(
        "repository",
        `https://github.com/${github_owner}/${project_name}`,
    );
}

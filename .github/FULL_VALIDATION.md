# Full-validation control

The `full-validation` label marks a pull request as being in final review.

While the label is present:

- `.github/workflows/full-validation.yml` runs the complete database and Rust
  validation after every new commit;
- `.github/workflows/package.yml` builds and verifies the complete release
  bundle when packaging paths are affected.

Create the label once in the repository with the exact name
`full-validation`. Keep it on the pull request until the exact merge head is
green. Remove it when returning the pull request to ordinary iterative work.

Production/security changes also require a manual run of the complete
`Production Foundation` workflow on the exact final branch head.

# Contributing
Any contributions welcome!

This repo uses `pre-commit` to run pre-commit and pre-push hooks. Installation instructions for `pre-commit` can be found on their website https://pre-commit.com
After cloning/forking the repo, please setup `pre-commit` in the repo using the following command:
```sh
pre-commit install --hook-type pre-commit --hook-type pre-push
```

Make sure you run `rustup update` inside of the repo so that `cargo-clippy` and `cargo-fmt` are supported within the repo.

Finally, please install [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks) which is used by one of the pre-commit hooks.

# CONTRIBUTING.md

First off, thank you for considering contributing to PicoForge! It's contributors like you that help make this project better for everyone. We deeply appreciate your time, effort, and interest in improving the tool.

This document outlines the guidelines and workflow for contributing to the `picoforge` project.



## Table of Contents

1. [Code Acquisition (Source Cloning)](#1-code-acquisition-source-cloning)
2. [Basic Workflow](#2-basic-workflow)
3. [Code Style and Formatting](#3-code-style-and-formatting)
4. [Pull Request (PR) Submission Workflow](#4-pull-request-pr-submission-workflow)
5. [Review Process](#5-review-process)
6. [Communication Channels](#6-communication-channels-discuss-before-contributing)
7. [Code of Conduct](#7-code-of-conduct)
8. [Legal Notice](#8-legal-notice)



## 1. Code Acquisition (Source Cloning)

The primary repository is hosted on GitHub, but a read-only mirror is available. **Mirrors can only be used to clone the source code locally.**

| Platform                      | URL                                              | Purpose                           |
| :---------------------------- | :----------------------------------------------- | :-------------------------------- |
| **GitHub (Primary Source)**   | `https://github.com/librekeys/picoforge.git`     | Required for Contribution Forking |
| **Personal Mirror**           | `https://git.suyogtandel.in/LibreKeys/picoforge` | Read-only viewing and cloning     |



## 2. Basic Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request directly against the `main` branch.



## 3. Code Style and Formatting

- Follow standard Rust coding guidelines and match the existing code style of the PicoForge source code.
- Always use `cargo fmt` to automatically format your code before committing.
- Write clear, concise commit messages.
- Always remember to update documentation for new features.
- Test on multiple platforms when possible before publishing changes.



## 4. Pull Request (PR) Submission Workflow

- Before submitting your Pull Request, please ensure you have followed the checklist in our [Pull Request Template](PULL_REQUEST_TEMPLATE.md).
- Ensure that any install or build dependencies are removed before the end of the layer when doing a build, and verify that your code compiles successfully.
- Before proceeding, please review the [general steps for completing a pull request](#2-basic-workflow) in this repository.
- One of the key aspects of validating a pull request is verifying that it doesn't break any existing work. Ensure that your code compiles, passes all tests, and that any CI checks run without errors.
- When opening a Pull Request (PR), please explicitly ask for a review from one of the maintainers. Depending on what is changed in the PR, tag either the repository or package maintainer.
- For more info on the project, please check [README.md](../README.md).



## 5. Review Process

- Our reviewers and maintainers contribute their free time to this project. Please be patient, as it may take a few days for them to review, approve, or request changes on your PR. We genuinely appreciate your work and want to ensure it integrates well into the project.
- In case your PR goes unanswered for more than 2 weeks, feel free to tag the main maintainer (`@lockedmutex`) in the comments to bring it to their attention.



## 6. Communication Channels (Discuss Before Contributing)

Matrix is our most preferred option for direct communication, followed by Discord.
- **Matrix**: [Join our Matrix room](https://matrix.to/#/%23librekeys:matrix.org)
- **Discord**: [Join our Discord server](https://discord.gg/6wYBpSHJY2)
- **Discussions**: [GitHub Discussions](https://github.com/librekeys/picoforge/discussions)
- **Issues**: [GitHub Issues](https://github.com/librekeys/picoforge/issues)

## 7. Code of Conduct

We are committed to providing a welcoming and inspiring community for all. Please be polite and respectful in all your interactions with the project.

## 8. Legal Notice

By contributing code, documentation, or other assets to the PicoForge project, the contributor agrees that the content is 100% authored by them, that they have the necessary rights to the content, and that the content may be provided under the project's existing license.

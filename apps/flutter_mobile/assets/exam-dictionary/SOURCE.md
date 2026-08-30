# Exam Corpus Dictionary Source

The generated `exam-corpus-dictionary.json` contains only entries matched by
the bundled exam corpus. Its primary source is ECDICT at commit
`bc015ed2e24a7abef49fc6dbbb7fe32c1dadaf8b`:

https://github.com/skywind3000/ECDICT

ECDICT is distributed under the MIT License copied into this directory.
Bundled seed-vocabulary books are used only as a fallback for matching terms.
Regenerate the asset with `scripts/build-exam-corpus-dictionary.mjs` and keep
`docs/exam-corpus-dictionary-report.json` as the coverage audit.

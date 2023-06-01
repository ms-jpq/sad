MAKEFLAGS += --jobs
MAKEFLAGS += --no-builtin-rules
MAKEFLAGS += --warn-undefined-variables
SHELL := bash
.DELETE_ON_ERROR:
.ONESHELL:
.SHELLFLAGS := -Eeuo pipefail -O dotglob -O failglob -O globstar -c

.DEFAULT_GOAL := help

.PHONY: clean clobber

clean:
	rm -rf --

clobber: clean
	rm -rf --

.venv/bin/pip:
	python3 -m venv -- .venv

.venv/bin/mypy: .venv/bin/pip
	'$<' install --upgrade --requirement requirements.txt -- mypy types-PyYAML types-toml types-Jinja2


.PHONY: lint

lint: .venv/bin/mypy
	'$<' -- .

.PHONY: deps

deps: .venv/bin/mypy
	.venv/bin/python3 ./build.py deps

.PHONY: build

build: .venv/bin/mypy
	.venv/bin/python3 ./build.py build

.PHONY: release

release: .venv/bin/mypy
	.venv/bin/python3 ./build.py buildr -- "$$TRIPLE"

.PHONY: ci

ci: .venv/bin/mypy
	.venv/bin/python3 ./ci/release.py

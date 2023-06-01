MAKEFLAGS += --jobs
MAKEFLAGS += --no-builtin-rules
MAKEFLAGS += --warn-undefined-variables
SHELL := bash
.DELETE_ON_ERROR:
.ONESHELL:
.SHELLFLAGS := -Eeuo pipefail -O dotglob -O nullglob -O failglob -O globstar -c

.DEFAULT_GOAL := help

.PHONY: clean clobber

ifeq ($(OS),Windows_NT)
VENV := Scripts
else
VENV := bin
endif

clean:
	shopt -u failglob
	rm -rf -- artifacts/*.{zip,deb} .mypy_cache/ target/ temp/

clobber: clean
	rm -rf -- .venv/

.venv/$(VENV)/pip:
	python3 -m venv -- .venv

.venv/$(VENV)/mypy: .venv/$(VENV)/pip
	'$<' install --upgrade --requirement requirements.txt -- mypy types-PyYAML types-toml types-Jinja2

.PHONY: lint

lint: .venv/$(VENV)/mypy
	'$<' -- .

.PHONY: deps

deps: .venv/$(VENV)/mypy
	.venv/$(VENV)/python3 ./build.py deps

.PHONY: build

build: .venv/$(VENV)/mypy
	.venv/$(VENV)/python3 ./build.py build

.PHONY: release

release: .venv/$(VENV)/mypy
	.venv/$(VENV)/python3 ./build.py buildr -- "$$TRIPLE"

.PHONY: ci

ci: .venv/$(VENV)/mypy
	.venv/$(VENV)/python3 ./ci/release.py

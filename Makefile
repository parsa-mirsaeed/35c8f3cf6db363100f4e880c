.DEFAULT_GOAL := help

.PHONY: help init dev up down logs ps migrate build package publish smoke validate clean

help init dev up down logs ps migrate smoke validate clean:
	@bash edutalent $@

build package publish:
	@bash edutalent $@ $(ARGS)

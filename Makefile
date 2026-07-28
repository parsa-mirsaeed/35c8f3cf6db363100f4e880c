.DEFAULT_GOAL := help

.PHONY: help init dev up down logs ps migrate build package publish appliance-build appliance-verify smoke validate clean \
	production-bootstrap production-init production-validate production-up \
	production-down production-logs production-ps production-migrate production-config \
	production-database-check production-gateway-check production-ai-check production-qdrant-check

help init dev up down logs ps migrate smoke validate clean \
production-bootstrap production-init production-validate production-up \
production-down production-logs production-ps production-migrate production-config \
production-database-check production-gateway-check production-ai-check production-qdrant-check:
	@bash edutalent $@

build package publish appliance-build appliance-verify:
	@bash edutalent $@ $(ARGS)

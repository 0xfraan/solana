# include .env file and export its env vars
# (-include to ignore error if it does not exist)
-include .env

# Variables
DOCKER_IMAGE_NAME ?= 0xfraan/price-function

check_docker_env:
ifeq ($(strip $(DOCKER_IMAGE_NAME)),)
	$(error DOCKER_IMAGE_NAME is not set)
else
	@echo DOCKER_IMAGE_NAME: ${DOCKER_IMAGE_NAME}
endif

docker_build: 
	docker buildx build --platform linux/amd64 --pull -f ./switchboard-function/Dockerfile -t ${DOCKER_IMAGE_NAME} --load ./switchboard-function
docker_publish: 
	docker buildx build --platform linux/amd64 --pull -f ./switchboard-function/Dockerfile -t ${DOCKER_IMAGE_NAME} --push ./switchboard-function

measurement: check_docker_env
	@docker run -d --platform=linux/amd64 -q --name=my-switchboard-function ${DOCKER_IMAGE_NAME}:latest
	@docker cp my-switchboard-function:/measurement.txt ./measurement.txt
	@echo -n 'MrEnclve: '
	@cat measurement.txt
	@docker stop my-switchboard-function > /dev/null
	@docker rm my-switchboard-function > /dev/null

anchor_build :; anchor build

build: anchor_build docker_build measurement

all: anchor_sync build

anchor_sync :; anchor keys sync

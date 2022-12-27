@echo off

:: First check if docker is running
docker ps >NUL 2>NUL
if %errorlevel% neq 0 (
  echo.
  echo Docker engine is not running! Run Docker Desktop application
  echo.
  exit
)

:: Check if unios-container exists and whether one should be created or started
docker container inspect unios-container >NUL 2>NUL
if %errorlevel% neq 0 (
  echo Haven't found container named unios-container
  echo Creating a new container from image yevhenii0/ubuntu-os:22.04.2. Dir %CD% will be mounted to /unios
  docker run -i -d --name unios-container -v %CD%:/unios yevhenii0/ubuntu-os:22.04.2
) else (
  docker start unios-container >NUL
)

:: Building kernel
echo Building kernel in the unios-container
docker exec unios-container bash -l -c "cd /unios && cargo run" 2>NUL

:: Running kernel is qemu emulator
qemu-system-x86_64 -drive format=raw,file=target/x86_64-my_os/debug/bootimage-unios.bin

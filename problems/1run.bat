@echo off
setlocal

set "SRC=..\target\debug\bootfrost.exe"
set "DST=bootfrost.exe"

copy /Y "%SRC%" "%DST%" >nul
if errorlevel 1 (
  echo Failed to copy "%SRC%" to "%CD%\%DST%"
  exit /b 1
)

".\%DST%" -s general -l 200 -f BlockMovePlanning.pcsf -j > BlockMovePlanning.log.pcsf
python ../tools/block_move_viewer/viewer.py

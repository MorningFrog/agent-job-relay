@echo off
setlocal DisableDelayedExpansion
if not defined AJR_STATE_DIR set "AJR_STATE_DIR=__AJR_INSTALLED_STATE__"
"__AJR_INSTALLED_EXECUTABLE__" %*
exit /b %errorlevel%

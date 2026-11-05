# LaunchServices gives the test real application/key-window ownership. CTest's
# direct executable launch can exercise protocols while leaving NSApp inactive.
if(NOT EXISTS "${SCARS_NATIVE_APP}/Contents/MacOS/scars_native_tests")
  message(FATAL_ERROR "Build scars_native_tests before running its foreground check")
endif()
set(_stdout "${SCARS_NATIVE_LOG_DIR}/native-foreground.stdout")
set(_stderr "${SCARS_NATIVE_LOG_DIR}/native-foreground.stderr")
file(REMOVE "${_stdout}" "${_stderr}")
execute_process(COMMAND /usr/bin/open -n -W --stdout "${_stdout}" --stderr "${_stderr}" "${SCARS_NATIVE_APP}"
  RESULT_VARIABLE _launch TIMEOUT 25 OUTPUT_VARIABLE _open_out ERROR_VARIABLE _open_err)
set(_out "")
set(_err "")
if(EXISTS "${_stdout}")
  file(READ "${_stdout}" _out)
endif()
if(EXISTS "${_stderr}")
  file(READ "${_stderr}" _err)
endif()
# open -W reports launch status, not the application's exit status. Require its
# post-assertion completion marker and actual foreground-only coverage markers.
if(NOT "${_launch}" STREQUAL "0" OR
   NOT _out MATCHES "SCARS native scrolling, accessibility, inspector, and lifetime checks passed" OR
   NOT _out MATCHES "Secure-input ownership enabled and released with password focus" OR
   NOT _out MATCHES "Secure-input ownership balanced across hide, unmount, and window close" OR
   NOT _out MATCHES "Native one-shot redraw delivered the debounced search without continuous animation" OR
   NOT _out MATCHES "Native logical accessibility verified 100000 rows with bounded realization" OR
   NOT _out MATCHES "Native tree outline, lazy children, disclosure, logical selection, and replacement checks passed" OR
   NOT _out MATCHES "Native table indexed columns/cells, selection, sorting, editing, IME, and schema lifetime checks passed" OR
   _out MATCHES "Frame settling requires an active desktop Space")
  message(FATAL_ERROR "SCARS foreground verification failed (${_launch})\n${_open_out}${_open_err}${_out}${_err}")
endif()
message(STATUS "${_out}")

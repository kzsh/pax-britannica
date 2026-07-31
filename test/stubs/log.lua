-- Headless stand-in for the native `log` module.

local log = {}

local quiet = os.getenv('DOKIDOKI_QUIET_LOG')

function log.log_message(message)
  if not quiet then
    io.stderr:write('log: ', tostring(message), '\n')
  end
end

return log

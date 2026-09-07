local rpc = { address = nil }
local queue = {}
local busy = false
local serial = 0

local function pump()
  if busy or not rpc.address or #queue == 0 then return end
  busy = true
  local request = table.remove(queue, 1)
  serial = serial + 1
  local id = serial
  local function finish(error)
    busy = false
    if request.callback then request.callback(error) end
    pump()
  end
  if not noctalia.http({
    url = rpc.address.url,
    method = "POST",
    headers = { "Content-Type: application/json", "Authorization: Bearer " .. rpc.address.token },
    body = noctalia.json.encode({ jsonrpc = "2.0", id = id, method = request.method, params = request.params }),
  }, function(response)
    local reply = response.ok and response.status == 200 and noctalia.json.decode(response.body)
    if type(reply) == "table" and reply.jsonrpc == "2.0" and reply.id == id then
      finish(reply.error)
    else
      finish({ code = -32098, message = "IPC request failed" })
    end
  end) then finish({ code = -32098, message = "IPC request could not start" }) end
end

function rpc.call(method, params, callback)
  queue[#queue + 1] = { method = method, params = params, callback = callback }
  pump()
end

function rpc.connect(address)
  rpc.address = address
  pump()
end

return rpc

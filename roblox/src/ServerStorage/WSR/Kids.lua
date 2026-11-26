-- The Lost: kids caught in the gang. You reach them by calling their name; then they follow you to the road.
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local ServerStorage = game:GetService("ServerStorage")
local Config = require(ReplicatedStorage:WaitForChild("WSRConfig"))
local WSR = ServerStorage:WaitForChild("WSR")
local Round = require(WSR:WaitForChild("Round"))
local PlayerState = require(WSR:WaitForChild("PlayerState"))
local NPC = require(WSR:WaitForChild("NPC"))

local M = {}
M.onChange = nil -- set by Main: function(free, safe, total)
M.onCalled = nil -- set by Main: function(player, kid)

local kids = {}

local function counts()
	local free, safe = 0, 0
	for _, k in ipairs(kids) do
		if k.state == "Follow" then
			free += 1
		elseif k.state == "Safe" then
			free += 1
			safe += 1
		end
	end
	return free, safe, #kids
end

local function changed()
	local free, safe, total = counts()
	workspace:SetAttribute("KidsFree", free)
	workspace:SetAttribute("KidsSafe", safe)
	if M.onChange then
		M.onChange(free, safe, total)
	end
end

local function insideBox(boxPart, pos)
	local p = boxPart.CFrame:PointToObjectSpace(pos)
	local h = boxPart.Size / 2
	return math.abs(p.X) <= h.X and math.abs(p.Y) <= h.Y + 4 and math.abs(p.Z) <= h.Z
end

function M.start(world, roundId)
	kids = {}
	local folder = Instance.new("Folder")
	folder.Name = "Lost"
	folder.Parent = world.folder

	for i, info in ipairs(Config.Kids) do
		local home = world.cells[i]
		local model, hum, hrp = NPC.make({
			name = info.name,
			parent = folder,
			pos = home,
			skin = Color3.fromRGB(120, 78, 44),
			shirt = Color3.fromRGB(150, 150, 160),
			pants = Color3.fromRGB(60, 70, 100),
			speed = 15,
			tag = true,
		})
		local kid = { info = info, model = model, hum = hum, hrp = hrp, home = home, state = "Locked", leader = nil }
		local prompt = Instance.new("ProximityPrompt")
		prompt.ActionText = "Call by name"
		prompt.ObjectText = info.name .. ", " .. info.age
		prompt.HoldDuration = Config.Hold.call
		prompt.MaxActivationDistance = 9
		prompt.RequiresLineOfSight = false
		prompt.Parent = hrp
		kid.prompt = prompt
		prompt.Triggered:Connect(function(player)
			if kid.state ~= "Locked" or not PlayerState.isActive(player) then
				return
			end
			kid.state = "Follow"
			kid.leader = player
			prompt.Enabled = false
			PlayerState.addWeight(player, -1) -- reaching someone lightens what you carry
			if M.onCalled then
				M.onCalled(player, kid)
			end
			changed()
		end)
		table.insert(kids, kid)

		task.spawn(function()
			while Round.id == roundId and model.Parent do
				if kid.state == "Follow" then
					local leaderRoot = kid.leader and PlayerState.getRoot(kid.leader)
					if not leaderRoot then
						M.release(kid)
					else
						local d = NPC.flatDist(hrp.Position, leaderRoot.Position)
						if insideBox(world.exit, hrp.Position) then
							kid.state = "Safe"
							kid.leader = nil
							hum:MoveTo(world.exit.Position + Vector3.new((i - 2) * 6, 0, 8))
							changed()
						elseif d > 80 then
							model:PivotTo(leaderRoot.CFrame * CFrame.new(3, 0, 3))
						elseif d > 7 then
							NPC.goTo(hum, hrp, leaderRoot.Position, function()
								return kid.state == "Follow" and Round.id == roundId
							end, 2)
						end
					end
				end
				task.wait(0.4)
			end
		end)
	end
	changed()
end

-- Back to the cell (the leader was caught or left).
function M.release(kid)
	if kid.state ~= "Follow" then
		return
	end
	kid.state = "Locked"
	kid.leader = nil
	kid.prompt.Enabled = true
	kid.hum:MoveTo(kid.home)
	changed()
end

function M.releaseFollowersOf(player)
	for _, kid in ipairs(kids) do
		if kid.leader == player then
			M.release(kid)
		end
	end
end

return M

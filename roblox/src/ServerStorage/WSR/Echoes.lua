-- Echoes: every point of Weight calls a faint figure that walks toward you. Touching one slows you for a moment.
-- They are not enemies you can fight; they are what you carry.
local Players = game:GetService("Players")
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local ServerStorage = game:GetService("ServerStorage")
local Config = require(ReplicatedStorage:WaitForChild("WSRConfig"))
local WSR = ServerStorage:WaitForChild("WSR")
local Round = require(WSR:WaitForChild("Round"))
local PlayerState = require(WSR:WaitForChild("PlayerState"))
local NPC = require(WSR:WaitForChild("NPC"))

local M = {}

local lists = {} -- [player] = array of echoes
local pending = {} -- [player] = number being built right now

local function ghostify(model)
	for _, d in ipairs(model:GetDescendants()) do
		if d:IsA("BasePart") then
			d.Material = Enum.Material.ForceField
			d.Color = Color3.fromRGB(150, 160, 190)
			if d.Name ~= "HumanoidRootPart" then
				d.CanCollide = false
			end
			d.CanQuery = false
		elseif d:IsA("Decal") then
			d.Transparency = 0.6
		end
	end
end

local function remove(player, echo)
	local list = lists[player]
	if list then
		local idx = table.find(list, echo)
		if idx then
			table.remove(list, idx)
		end
	end
	if echo.model then
		echo.model:Destroy()
	end
end

function M.start(world, roundId)
	lists = {}
	pending = {}
	local folder = Instance.new("Folder")
	folder.Name = "Echoes"
	folder.Parent = world.folder

	local function sync(player)
		if Round.id ~= roundId then
			return
		end
		local target = math.min(PlayerState.get(player).weight, Config.Weight.maxEchoes)
		lists[player] = lists[player] or {}
		pending[player] = pending[player] or 0
		local list = lists[player]
		while #list > target do
			remove(player, list[#list])
		end
		while #list + pending[player] < target do
			pending[player] += 1
			task.spawn(function()
				-- Pick a spawn point away from the player.
				local root = PlayerState.getRoot(player)
				local options = {}
				for _, s in ipairs(world.echoSpawns) do
					if not root or (s - root.Position).Magnitude > 30 then
						table.insert(options, s)
					end
				end
				local pos = options[math.random(1, math.max(#options, 1))] or world.echoSpawns[1]
				local model, hum, hrp = NPC.make({
					name = "Echo",
					parent = folder,
					pos = pos,
					skin = Color3.fromRGB(150, 160, 190),
					shirt = Color3.fromRGB(150, 160, 190),
					pants = Color3.fromRGB(150, 160, 190),
					speed = Config.Weight.echoSpeed,
					tag = false,
				})
				ghostify(model)
				pending[player] -= 1
				if Round.id ~= roundId or not player.Parent then
					model:Destroy()
					return
				end
				table.insert(list, { model = model, hum = hum, hrp = hrp })
			end)
		end
	end

	PlayerState.onWeight(function(player)
		sync(player)
	end)

	task.spawn(function()
		while Round.id == roundId do
			for player, list in pairs(lists) do
				local root = PlayerState.getRoot(player)
				if root and PlayerState.isActive(player) then
					for _, echo in ipairs({ table.unpack(list) }) do
						if echo.hrp and echo.hrp.Parent then
							if NPC.flatDist(echo.hrp.Position, root.Position) <= Config.Weight.echoTouch then
								PlayerState.slow(player, Config.Weight.echoSlowSeconds)
								remove(player, echo)
								task.delay(Config.Weight.echoRespawnSeconds, sync, player)
							else
								echo.hum:MoveTo(root.Position)
							end
						end
					end
				end
			end
			task.wait(0.25)
		end
	end)
end

return M

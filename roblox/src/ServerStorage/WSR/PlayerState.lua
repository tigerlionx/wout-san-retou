-- Per-player state that must be decided on the server: stance, speed, Weight, flash cooldown and detention.
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Config = require(ReplicatedStorage:WaitForChild("WSRConfig"))

local M = {}
local states = {}
local weightListeners = {}

function M.get(player)
	local s = states[player]
	if not s then
		s = { weight = 0, stance = "walk", flashReadyAt = 0, detainedUntil = 0, slowUntil = 0 }
		states[player] = s
		player:SetAttribute("Weight", 0)
		player:SetAttribute("Stance", "walk")
	end
	return s
end

function M.remove(player)
	states[player] = nil
end

function M.getHumanoid(player)
	local char = player.Character
	return char and char:FindFirstChildOfClass("Humanoid")
end

function M.getRoot(player)
	local char = player.Character
	return char and char:FindFirstChild("HumanoidRootPart")
end

-- True while the player is alive, in the game, and not being held by the gang.
function M.isActive(player)
	local hum = M.getHumanoid(player)
	if not hum or hum.Health <= 0 or not M.getRoot(player) then
		return false
	end
	return M.get(player).detainedUntil <= os.clock()
end

function M.applySpeed(player)
	local hum = M.getHumanoid(player)
	if not hum then
		return
	end
	local s = M.get(player)
	local now = os.clock()
	if s.detainedUntil > now then
		hum.WalkSpeed = 0
		return
	end
	local mult = math.max(Config.Weight.minMultiplier, 1 - Config.Weight.slowPerPoint * s.weight)
	if s.slowUntil > now then
		mult = mult * Config.Weight.echoSlowMultiplier
	end
	hum.WalkSpeed = Config.Speed[s.stance] * mult
end

function M.setStance(player, stance)
	if Config.Speed[stance] == nil then
		return
	end
	local s = M.get(player)
	s.stance = stance
	player:SetAttribute("Stance", stance)
	M.applySpeed(player)
end

function M.onWeight(fn)
	table.insert(weightListeners, fn)
end

function M.clearListeners()
	weightListeners = {}
end

function M.addWeight(player, delta)
	local s = M.get(player)
	s.weight = math.max(0, s.weight + delta)
	player:SetAttribute("Weight", s.weight)
	M.applySpeed(player)
	for _, fn in ipairs(weightListeners) do
		task.spawn(fn, player, s.weight)
	end
end

function M.slow(player, seconds)
	local s = M.get(player)
	s.slowUntil = os.clock() + seconds
	M.applySpeed(player)
	task.delay(seconds + 0.05, function()
		if player.Parent then
			M.applySpeed(player)
		end
	end)
end

-- Start of a new round: everyone begins with no Weight.
function M.reset()
	for player, s in pairs(states) do
		s.weight = 0
		s.stance = "walk"
		s.flashReadyAt = 0
		s.detainedUntil = 0
		s.slowUntil = 0
		player:SetAttribute("Weight", 0)
		player:SetAttribute("Stance", "walk")
	end
end

return M

-- Wout San Retou: The Road of No Return (Roblox edition). Server entry point: builds the round and runs the rules.
--
-- The story, kept non-violent for Roblox: it is night in a gang's compound. Cut the power, get into the cellblock, call the
-- Lost kids by name and lead them out by the road. Guards catch you and send you back; the flash can blind them, but
-- using force adds Weight, and Weight calls Echoes that slow you down. Reaching the kids by name lightens it.
local Players = game:GetService("Players")
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local ServerStorage = game:GetService("ServerStorage")
local Lighting = game:GetService("Lighting")
local TweenService = game:GetService("TweenService")

local Config = require(ReplicatedStorage:WaitForChild("WSRConfig"))
local WSR = ServerStorage:WaitForChild("WSR")
local Round = require(WSR:WaitForChild("Round"))
local PlayerState = require(WSR:WaitForChild("PlayerState"))
local World = require(WSR:WaitForChild("World"))
local Guards = require(WSR:WaitForChild("Guards"))
local Kids = require(WSR:WaitForChild("Kids"))
local Echoes = require(WSR:WaitForChild("Echoes"))

-- Remote events the client listens to (server to client) or fires (client to server).
local remotes = ReplicatedStorage:FindFirstChild("WSRRemotes")
if not remotes then
	remotes = Instance.new("Folder")
	remotes.Name = "WSRRemotes"
	remotes.Parent = ReplicatedStorage
	for _, name in ipairs({ "Stance", "Flash", "Notify", "Detained", "Ending" }) do
		local r = Instance.new("RemoteEvent")
		r.Name = name
		r.Parent = remotes
	end
end

local function notifyAll(text)
	remotes.Notify:FireAllClients(text)
end

local function setupLighting()
	Lighting.ClockTime = 0
	Lighting.Brightness = 0.4
	Lighting.Ambient = Color3.fromRGB(28, 30, 46)
	Lighting.OutdoorAmbient = Color3.fromRGB(38, 40, 62)
	Lighting.FogColor = Color3.fromRGB(14, 16, 26)
	Lighting.FogStart = 60
	Lighting.FogEnd = 260
	Lighting.GlobalShadows = true
	if not Lighting:FindFirstChild("WSR_Atmosphere") then
		local a = Instance.new("Atmosphere")
		a.Name = "WSR_Atmosphere"
		a.Density = 0.3
		a.Color = Color3.fromRGB(40, 46, 70)
		a.Parent = Lighting
		local cc = Instance.new("ColorCorrectionEffect")
		cc.Name = "WSR_Color"
		cc.Saturation = -0.25
		cc.Contrast = 0.1
		cc.Parent = Lighting
		local bloom = Instance.new("BloomEffect")
		bloom.Name = "WSR_Bloom"
		bloom.Intensity = 0.5
		bloom.Parent = Lighting
	end
end

local currentWorld = nil

local function endRound()
	if workspace:GetAttribute("RoundOver") then
		return
	end
	workspace:SetAttribute("RoundOver", true)
	local total, n = 0, 0
	for _, p in ipairs(Players:GetPlayers()) do
		total += PlayerState.get(p).weight
		n += 1
	end
	local avg = n > 0 and total / n or 0
	local text
	if avg <= 1 then
		text = "You carried them out and left nothing heavy behind."
	elseif avg <= 4 then
		text = "You got them out. Some of the weight came with you."
	else
		text = "They are free. What it cost to get here is still with you."
	end
	remotes.Ending:FireAllClients(text, workspace:GetAttribute("KidsSafe") or 0)
	task.delay(15, function()
		-- startRound is defined below; this closure only runs after the file has loaded.
		Round.restart()
	end)
end

local function connectGenerator(world, gen)
	gen.prompt.Triggered:Connect(function()
		if not gen.on then
			return
		end
		gen.on = false
		gen.prompt.Enabled = false
		gen.glow.Enabled = false
		gen.part.Color = Color3.fromRGB(70, 60, 40)
		for _, lamp in ipairs(world.lamps) do
			if lamp.group == gen.group then
				lamp.light.Enabled = false
				lamp.bulb.Material = Enum.Material.SmoothPlastic
				lamp.bulb.Color = Color3.fromRGB(40, 40, 44)
			end
		end
		local off = (workspace:GetAttribute("GensOff") or 0) + 1
		workspace:SetAttribute("GensOff", off)
		if off >= #world.gens then
			notifyAll("All the power is cut. The cellblock door is open.")
			local tween = TweenService:Create(world.door, TweenInfo.new(2), { CFrame = world.door.CFrame + Vector3.new(0, 13, 0) })
			tween:Play()
			tween.Completed:Connect(function()
				world.door.CanCollide = false
			end)
		else
			notifyAll("Generator " .. gen.group .. " is off. " .. off .. " of " .. #world.gens .. ".")
		end
	end)
end

local function startRound()
	Round.id += 1
	local id = Round.id
	PlayerState.clearListeners()
	PlayerState.reset()
	local old = workspace:FindFirstChild("WSR_World")
	if old then
		old:Destroy()
	end

	local world = World.build()
	currentWorld = world
	workspace:SetAttribute("GensOff", 0)
	workspace:SetAttribute("GensTotal", #world.gens)
	workspace:SetAttribute("KidsFree", 0)
	workspace:SetAttribute("KidsSafe", 0)
	workspace:SetAttribute("KidsTotal", #Config.Kids)
	workspace:SetAttribute("Alert", 0)
	workspace:SetAttribute("RoundOver", false)

	for _, gen in ipairs(world.gens) do
		connectGenerator(world, gen)
	end

	Kids.onCalled = function(player, kid)
		remotes.Notify:FireClient(player, kid.info.line)
	end
	Kids.onChange = function(free, safe, total)
		if safe >= total and total > 0 then
			endRound()
		end
	end
	Guards.onCaught = function(player)
		local st = PlayerState.get(player)
		if os.clock() < st.detainedUntil or workspace:GetAttribute("RoundOver") then
			return
		end
		st.detainedUntil = os.clock() + 3
		PlayerState.applySpeed(player)
		remotes.Detained:FireClient(player, "Caught. They drag you back to the road.")
		Kids.releaseFollowersOf(player)
		Guards.calmAll()
		task.delay(1.2, function()
			if currentWorld == world and player.Character then
				player.Character:PivotTo(world.spawn.CFrame + Vector3.new(0, 5, 0))
			end
		end)
		task.delay(3, function()
			st.detainedUntil = 0
			PlayerState.applySpeed(player)
		end)
	end

	Guards.start(world, id)
	Kids.start(world, id)
	Echoes.start(world, id)

	for _, p in ipairs(Players:GetPlayers()) do
		p:LoadCharacter()
	end
	notifyAll("Cut the power. Reach the cellblock. Call them by name. Lead them to the road.")
end
Round.restart = startRound

-- Player setup.
local function onPlayer(player)
	PlayerState.get(player)
	player.CharacterAdded:Connect(function(char)
		local s = PlayerState.get(player)
		s.stance = "walk"
		s.detainedUntil = 0
		player:SetAttribute("Stance", "walk")
		char:WaitForChild("Humanoid")
		PlayerState.applySpeed(player)
	end)
end

Players.PlayerAdded:Connect(onPlayer)
Players.PlayerRemoving:Connect(PlayerState.remove)
for _, p in ipairs(Players:GetPlayers()) do
	onPlayer(p)
end

remotes.Stance.OnServerEvent:Connect(function(player, stance)
	if type(stance) == "string" then
		PlayerState.setStance(player, stance)
	end
end)

remotes.Flash.OnServerEvent:Connect(function(player)
	if not PlayerState.isActive(player) or workspace:GetAttribute("RoundOver") then
		return
	end
	local s = PlayerState.get(player)
	local now = os.clock()
	if now < s.flashReadyAt then
		return
	end
	local hit = Guards.flash(player)
	if hit > 0 then
		s.flashReadyAt = now + Config.Flash.cooldown
		PlayerState.addWeight(player, Config.Flash.weightCost)
		remotes.Notify:FireClient(player, "You blinded someone. It stays with you.")
	else
		s.flashReadyAt = now + Config.Flash.missCooldown
		remotes.Notify:FireClient(player, "Nobody in front of you.")
	end
	remotes.Flash:FireClient(player, hit, s.flashReadyAt - now)
end)

setupLighting()
Players.RespawnTime = 3
startRound()

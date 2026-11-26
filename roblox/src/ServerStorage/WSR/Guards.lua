-- The gang's guards. They patrol, notice you by sight and sound, investigate, chase and give up.
-- They never hurt anyone: being caught sends you back to the road (see Main).
local Players = game:GetService("Players")
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Config = require(ReplicatedStorage:WaitForChild("WSRConfig"))
local ServerStorage = game:GetService("ServerStorage")
local WSR = ServerStorage:WaitForChild("WSR")
local Round = require(WSR:WaitForChild("Round"))
local PlayerState = require(WSR:WaitForChild("PlayerState"))
local NPC = require(WSR:WaitForChild("NPC"))

local M = {}
M.onCaught = nil -- set by Main: function(player)

local guards = {}
local npcFolder = nil

local function setState(g, state)
	if g.state ~= state then
		g.state = state
		g.token += 1
	end
end

local function lightMultiplier()
	local off = workspace:GetAttribute("GensOff") or 0
	return math.max(0.3, 1 - Config.DarknessPerGenerator * off)
end

local function hasLineOfSight(g, targetPos, char)
	local origin = g.head.Position
	local result = workspace:Raycast(origin, targetPos - origin, g.rayParams)
	return result == nil or (char ~= nil and result.Instance:IsDescendantOf(char))
end

-- One look around. Updates awareness (0 to 1) and decides the next state.
local function sense(g, dt)
	local now = os.clock()
	if g.state == "Stunned" then
		if now >= g.stunnedUntil then
			g.awareness = 0.2
			setState(g, "Patrol")
		end
		return 0
	end
	if not g.head or not g.head.Parent then
		return 0
	end

	local seenPlayer, seenDist, seenRange, heardPos
	local range = 0
	for _, p in ipairs(Players:GetPlayers()) do
		if PlayerState.isActive(p) then
			local root = PlayerState.getRoot(p)
			local stance = PlayerState.get(p).stance
			local offset = root.Position - g.head.Position
			local dist = offset.Magnitude
			range = Config.Guard.sightRange * Config.StanceSight[stance] * lightMultiplier()
			local visible = false
			if dist <= math.max(range, Config.Guard.closeSense) then
				local close = dist <= Config.Guard.closeSense
				local cosAngle = dist > 0.01 and g.head.CFrame.LookVector:Dot(offset.Unit) or 1
				if close or cosAngle >= math.cos(math.rad(Config.Guard.sightHalfAngle)) then
					visible = hasLineOfSight(g, root.Position, p.Character)
				end
			end
			if visible and (not seenDist or dist < seenDist) then
				seenPlayer, seenDist, seenRange = p, dist, range
			end
			local flatSpeed = Vector3.new(root.AssemblyLinearVelocity.X, 0, root.AssemblyLinearVelocity.Z).Magnitude
			if flatSpeed > 3 and dist <= Config.Hearing[stance] then
				heardPos = root.Position
			end
		end
	end

	if seenPlayer then
		local closeness = 1 - math.clamp(seenDist / math.max(seenRange, 1), 0, 1)
		g.awareness += dt * (0.7 + 1.2 * closeness)
		g.lastKnown = PlayerState.getRoot(seenPlayer).Position
		g.lastSeen = now
		g.target = seenPlayer
	elseif heardPos then
		g.awareness += dt * 0.45
		g.lastKnown = heardPos
	else
		g.awareness -= dt * 0.2
	end
	g.awareness = math.clamp(g.awareness, 0, 1)

	if g.state == "Chase" then
		if seenPlayer and seenDist <= Config.Guard.catchDistance and M.onCaught then
			M.onCaught(seenPlayer)
		elseif now - g.lastSeen > Config.Guard.loseSeconds then
			g.awareness = 0.5
			setState(g, "Search")
		end
	else
		if g.awareness >= 1 then
			setState(g, "Chase")
		elseif g.awareness >= 0.35 and g.state == "Patrol" then
			setState(g, "Investigate")
		elseif g.awareness < 0.1 and g.state == "Investigate" then
			setState(g, "Patrol")
		end
	end
	return g.awareness
end

local function stillValid(g, token, roundId)
	return Round.id == roundId and g.token == token
end

-- The movement thread: acts on whatever state the senses have chosen.
local function run(g, roundId)
	while Round.id == roundId and g.model.Parent do
		local state, token = g.state, g.token
		local valid = function()
			return stillValid(g, token, roundId)
		end
		if state == "Stunned" then
			g.hum.WalkSpeed = 0
			task.wait(0.2)
		elseif state == "Patrol" then
			g.hum.WalkSpeed = Config.Guard.patrolSpeed
			NPC.goTo(g.hum, g.hrp, g.route[g.routeIndex], valid, 25)
			if valid() then
				g.routeIndex = g.routeIndex % #g.route + 1
				local t0 = os.clock()
				while valid() and os.clock() - t0 < 1 do
					task.wait(0.1)
				end
			end
		elseif state == "Investigate" or state == "Search" then
			g.hum.WalkSpeed = Config.Guard.investigateSpeed
			NPC.goTo(g.hum, g.hrp, g.lastKnown, valid, 12)
			if valid() then
				local t0 = os.clock()
				while valid() and os.clock() - t0 < 3 do
					task.wait(0.1)
				end
				if valid() then
					g.awareness = math.min(g.awareness, 0.2)
					setState(g, "Patrol")
				end
			end
		elseif state == "Chase" then
			g.hum.WalkSpeed = Config.Guard.chaseSpeed
			if os.clock() - g.lastSeen < 0.5 then
				g.hum:MoveTo(g.lastKnown)
				task.wait(0.12)
			else
				NPC.goTo(g.hum, g.hrp, g.lastKnown, valid, 4)
			end
		else
			task.wait(0.2)
		end
	end
end

function M.start(world, roundId)
	guards = {}
	npcFolder = Instance.new("Folder")
	npcFolder.Name = "Guards"
	npcFolder.Parent = world.folder
	local rayParams = RaycastParams.new()
	rayParams.FilterType = Enum.RaycastFilterType.Exclude
	rayParams.FilterDescendantsInstances = { npcFolder }

	for i, route in ipairs(world.routes) do
		local model, hum, hrp = NPC.make({
			name = "Guard " .. i,
			parent = npcFolder,
			pos = route[1],
			skin = Color3.fromRGB(141, 85, 36),
			shirt = Color3.fromRGB(40, 40, 46),
			pants = Color3.fromRGB(30, 30, 34),
			speed = Config.Guard.patrolSpeed,
			tag = false,
		})
		local head = model:FindFirstChild("Head")
		if head then
			local lamp = Instance.new("SpotLight")
			lamp.Face = Enum.NormalId.Front
			lamp.Angle = 55
			lamp.Range = 40
			lamp.Brightness = 3
			lamp.Color = Color3.fromRGB(255, 235, 190)
			lamp.Parent = head
		end
		local g = {
			model = model, hum = hum, hrp = hrp, head = head or hrp, route = route, routeIndex = 2,
			state = "Patrol", token = 0, awareness = 0, lastKnown = route[1], lastSeen = 0,
			stunnedUntil = 0, rayParams = rayParams,
		}
		table.insert(guards, g)
		task.spawn(run, g, roundId)
	end

	-- One shared thread for the senses, ten times a second.
	task.spawn(function()
		local dt = 0.1
		while Round.id == roundId do
			local highest = 0
			for _, g in ipairs(guards) do
				highest = math.max(highest, sense(g, dt))
			end
			workspace:SetAttribute("Alert", highest)
			task.wait(dt)
		end
	end)
end

-- Everyone forgets the chase (used after a player is caught).
function M.calmAll()
	for _, g in ipairs(guards) do
		if g.state ~= "Stunned" then
			g.awareness = 0
			setState(g, "Patrol")
		end
	end
end

-- The flash: blinds guards in front of the player, within range and in plain sight. Returns how many.
function M.flash(player)
	local root = PlayerState.getRoot(player)
	if not root then
		return 0
	end
	local count = 0
	local cosLimit = math.cos(math.rad(Config.Flash.halfAngle))
	for _, g in ipairs(guards) do
		if g.state ~= "Stunned" and g.hrp and g.hrp.Parent then
			local offset = g.hrp.Position - root.Position
			local dist = offset.Magnitude
			if dist <= Config.Flash.range and dist > 0.01 and root.CFrame.LookVector:Dot(offset.Unit) >= cosLimit then
				local params = RaycastParams.new()
				params.FilterType = Enum.RaycastFilterType.Exclude
				params.FilterDescendantsInstances = { player.Character }
				local result = workspace:Raycast(root.Position, offset, params)
				if result == nil or result.Instance:IsDescendantOf(g.model) then
					g.stunnedUntil = os.clock() + Config.Flash.stunSeconds
					setState(g, "Stunned")
					g.hum:MoveTo(g.hrp.Position)
					count += 1
				end
			end
		end
	end
	return count
end

return M

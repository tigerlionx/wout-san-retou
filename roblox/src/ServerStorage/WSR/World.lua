-- Builds the compound in code: yard, walls, two buildings, the cellblock, generators, lamps, cover and the road out.
-- Layout (top view, -Z is north): the road and spawn are south (+Z), the cellblock is in the far north.
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Config = require(ReplicatedStorage:WaitForChild("WSRConfig"))

local M = {}

local CONCRETE = Color3.fromRGB(58, 60, 66)
local WALL = Color3.fromRGB(46, 48, 54)
local METAL = Color3.fromRGB(70, 74, 82)

local function part(parent, name, size, cf, color, material)
	local p = Instance.new("Part")
	p.Name = name
	p.Anchored = true
	p.Size = size
	p.CFrame = cf
	p.Color = color or CONCRETE
	p.Material = material or Enum.Material.Concrete
	p.TopSurface = Enum.SurfaceType.Smooth
	p.BottomSurface = Enum.SurfaceType.Smooth
	p.Parent = parent
	return p
end

-- A wall that runs along X at depth z, from x1 to x2, with an optional gap (a doorway) centred at gapCenter.
local function wallX(parent, z, x1, x2, h, gapCenter, gapW, color)
	local function seg(a, b)
		if b - a > 0.1 then
			part(parent, "Wall", Vector3.new(b - a, h, 1.5), CFrame.new((a + b) / 2, h / 2, z), color or WALL)
		end
	end
	if gapCenter then
		seg(x1, gapCenter - gapW / 2)
		seg(gapCenter + gapW / 2, x2)
	else
		seg(x1, x2)
	end
end

-- A wall that runs along Z at x, from z1 to z2, with an optional gap.
local function wallZ(parent, x, z1, z2, h, gapCenter, gapW, color)
	local function seg(a, b)
		if b - a > 0.1 then
			part(parent, "Wall", Vector3.new(1.5, h, b - a), CFrame.new(x, h / 2, (a + b) / 2), color or WALL)
		end
	end
	if gapCenter then
		seg(z1, gapCenter - gapW / 2)
		seg(gapCenter + gapW / 2, z2)
	else
		seg(z1, z2)
	end
end

-- A walled building with a roof and one doorway on the side named by doorSide ("N", "S", "E", "W").
local function room(parent, cx, cz, w, d, h, doorSide, doorW)
	local x1, x2, z1, z2 = cx - w / 2, cx + w / 2, cz - d / 2, cz + d / 2
	wallX(parent, z1, x1, x2, h, doorSide == "N" and cx or nil, doorW)
	wallX(parent, z2, x1, x2, h, doorSide == "S" and cx or nil, doorW)
	wallZ(parent, x1, z1, z2, h, doorSide == "W" and cz or nil, doorW)
	wallZ(parent, x2, z1, z2, h, doorSide == "E" and cz or nil, doorW)
	part(parent, "Roof", Vector3.new(w + 1.5, 1, d + 1.5), CFrame.new(cx, h + 0.5, cz), Color3.fromRGB(38, 40, 46))
end

local function inRect(x, z, r, margin)
	return x > r[1] - margin and x < r[2] + margin and z > r[3] - margin and z < r[4] + margin
end

function M.build()
	local folder = Instance.new("Folder")
	folder.Name = "WSR_World"
	folder.Parent = workspace

	-- Ground: the whole compound plus the road to the south.
	part(folder, "Ground", Vector3.new(320, 2, 320), CFrame.new(0, -1, 20), Color3.fromRGB(34, 35, 38), Enum.Material.Asphalt)
	part(folder, "RoadStripe", Vector3.new(3, 0.2, 60), CFrame.new(0, 0.1, 128), Color3.fromRGB(190, 170, 70), Enum.Material.SmoothPlastic)

	-- Perimeter, with a gate gap on the south wall.
	local H = 16
	wallX(folder, -100, -100, 100, H)
	wallX(folder, 100, -100, 100, H, 0, 26)
	wallZ(folder, -100, -100, 100, H)
	wallZ(folder, 100, -100, 100, H)

	-- Buildings.
	local warehouseRect = { -85, -35, -5, 45 }
	local houseRect = { 35, 85, -30, 30 }
	local cellRect = { -40, 40, -95, -60 }
	room(folder, -60, 20, 50, 50, 12, "E", 8)
	room(folder, 60, 0, 50, 60, 12, "W", 8)
	room(folder, 0, -77.5, 80, 35, 12, "S", 8)

	-- Cellblock: two partitions make three cells; the steel door stays shut until the power is cut.
	wallZ(folder, -13, -95, -72, 12)
	wallZ(folder, 13, -95, -72, 12)
	local door = part(folder, "CellDoor", Vector3.new(8, 12, 1.5), CFrame.new(0, 6, -60), METAL, Enum.Material.DiamondPlate)
	local cells = { Vector3.new(-26, 3, -86), Vector3.new(0, 3, -86), Vector3.new(26, 3, -86) }

	-- Spawn on the road, outside the gate.
	local spawn = Instance.new("SpawnLocation")
	spawn.Name = "RoadSpawn"
	spawn.Anchored = true
	spawn.Size = Vector3.new(14, 1, 14)
	spawn.CFrame = CFrame.new(0, 0.5, 135)
	spawn.Neutral = true
	spawn.Duration = 0
	spawn.Transparency = 1
	spawn.CanCollide = false
	spawn.Parent = folder

	-- The road out: kids who stand here are safe.
	local exit = part(folder, "ExitZone", Vector3.new(40, 10, 34), CFrame.new(0, 5, 114), Color3.fromRGB(90, 200, 120), Enum.Material.SmoothPlastic)
	exit.Transparency = 1
	exit.CanCollide = false
	exit.CanQuery = false
	exit.CanTouch = false
	local sign = part(folder, "RoadSign", Vector3.new(26, 4, 1), CFrame.new(0, 14, 100), Color3.fromRGB(20, 24, 22), Enum.Material.SmoothPlastic)
	local gui = Instance.new("SurfaceGui")
	gui.Face = Enum.NormalId.Front
	gui.Parent = sign
	local label = Instance.new("TextLabel")
	label.Size = UDim2.fromScale(1, 1)
	label.BackgroundTransparency = 1
	label.Text = "THE ROAD OUT"
	label.TextScaled = true
	label.Font = Enum.Font.GothamBold
	label.TextColor3 = Color3.fromRGB(120, 230, 150)
	label.Parent = gui
	local sign2 = sign:Clone()
	sign2.CFrame = sign.CFrame * CFrame.Angles(0, math.pi, 0)
	sign2.Parent = folder

	-- Generators: cutting all three kills the lights and opens the cellblock.
	local genSpots = { Vector3.new(-75, 2.5, 75), Vector3.new(75, 2.5, 55), Vector3.new(-72, 2.5, 28) }
	local gens = {}
	for i, pos in ipairs(genSpots) do
		local g = part(folder, "Generator" .. i, Vector3.new(6, 5, 6), CFrame.new(pos), Color3.fromRGB(200, 150, 40), Enum.Material.Metal)
		local prompt = Instance.new("ProximityPrompt")
		prompt.ActionText = "Cut the power"
		prompt.ObjectText = "Generator " .. i
		prompt.HoldDuration = Config.Hold.generator
		prompt.MaxActivationDistance = 9
		prompt.RequiresLineOfSight = false
		prompt.Parent = g
		local glow = Instance.new("PointLight")
		glow.Color = Color3.fromRGB(255, 190, 80)
		glow.Range = 14
		glow.Brightness = 1.5
		glow.Parent = g
		gens[i] = { part = g, prompt = prompt, glow = glow, group = i, on = true }
	end

	-- Lamps. Each belongs to its nearest generator and goes dark when that one is cut.
	local lampSpots = {
		{ -70, 85 }, { 0, 85 }, { 70, 85 }, { -90, 55 }, { 20, 45 }, { -20, 15 }, { 90, 40 },
		{ 20, -10 }, { 90, -45 }, { -90, -45 }, { -20, -45 }, { 20, -45 }, { -60, 20 }, { 60, 0 },
	}
	local lamps = {}
	for _, s in ipairs(lampSpots) do
		local x, z = s[1], s[2]
		local pole = part(folder, "LampPole", Vector3.new(0.6, 14, 0.6), CFrame.new(x, 7, z), METAL, Enum.Material.Metal)
		local bulb = part(folder, "LampBulb", Vector3.new(2, 2, 2), CFrame.new(x, 14.5, z), Color3.fromRGB(255, 214, 140), Enum.Material.Neon)
		bulb.Shape = Enum.PartType.Ball
		bulb.CanCollide = false
		local light = Instance.new("PointLight")
		light.Color = Color3.fromRGB(255, 200, 130)
		light.Range = 38
		light.Brightness = 1.6
		light.Parent = bulb
		local best, bestD = 1, math.huge
		for i, pos in ipairs(genSpots) do
			local d = (Vector3.new(x, 0, z) - Vector3.new(pos.X, 0, pos.Z)).Magnitude
			if d < bestD then
				best, bestD = i, d
			end
		end
		table.insert(lamps, { light = light, bulb = bulb, group = best })
		pole.CanCollide = false
	end

	-- Cover: crates scattered in the yard, away from the buildings and the road.
	local rng = Random.new(42)
	local placed = 0
	local tries = 0
	while placed < 36 and tries < 400 do
		tries += 1
		local x, z = rng:NextInteger(-92, 92), rng:NextInteger(-55, 92)
		if
			not inRect(x, z, warehouseRect, 6)
			and not inRect(x, z, houseRect, 6)
			and not inRect(x, z, cellRect, 6)
			and not (math.abs(x) < 16 and z > 80)
		then
			local s = rng:NextInteger(4, 7)
			part(folder, "Crate", Vector3.new(s, s, s), CFrame.new(x, s / 2, z) * CFrame.Angles(0, rng:NextNumber(0, math.pi), 0), Color3.fromRGB(86, 64, 40), Enum.Material.Wood)
			placed += 1
		end
	end
	-- A few crates inside the warehouse and the house, for hiding.
	for _, c in ipairs({ { -50, 10 }, { -70, 38 }, { -45, 35 }, { 50, -15 }, { 70, 15 }, { 55, 20 } }) do
		part(folder, "Crate", Vector3.new(5, 5, 5), CFrame.new(c[1], 2.5, c[2]), Color3.fromRGB(86, 64, 40), Enum.Material.Wood)
	end

	local routes = {
		{ Vector3.new(-60, 3, 85), Vector3.new(60, 3, 85), Vector3.new(60, 3, 65), Vector3.new(-60, 3, 65) }, -- the yard by the gate
		{ Vector3.new(-90, 3, -55), Vector3.new(-50, 3, -55), Vector3.new(-50, 3, -30), Vector3.new(-90, 3, -30) }, -- west side
		{ Vector3.new(90, 3, -55), Vector3.new(50, 3, -55), Vector3.new(50, 3, -40), Vector3.new(90, 3, -40) }, -- east side
		{ Vector3.new(-20, 3, 50), Vector3.new(20, 3, 50), Vector3.new(20, 3, 10), Vector3.new(-20, 3, 10) }, -- the middle
		{ Vector3.new(-75, 3, 0), Vector3.new(-45, 3, 0), Vector3.new(-45, 3, 40), Vector3.new(-75, 3, 40) }, -- inside the warehouse
		{ Vector3.new(-30, 3, -48), Vector3.new(30, 3, -48), Vector3.new(30, 3, -30), Vector3.new(-30, 3, -30) }, -- in front of the cellblock
	}

	local echoSpawns = {
		Vector3.new(-90, 3, -40), Vector3.new(90, 3, -50), Vector3.new(0, 3, 10), Vector3.new(-20, 3, 50), Vector3.new(25, 3, -40),
	}

	return {
		folder = folder,
		spawn = spawn,
		exit = exit,
		door = door,
		cells = cells,
		gens = gens,
		lamps = lamps,
		routes = routes,
		echoSpawns = echoSpawns,
	}
end

return M

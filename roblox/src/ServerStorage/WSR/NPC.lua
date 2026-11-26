-- Builds people (guards, kids, echoes) and walks them around with pathfinding.
local Players = game:GetService("Players")
local PathfindingService = game:GetService("PathfindingService")

local M = {}

local function flatDist(a, b)
	return Vector3.new(a.X - b.X, 0, a.Z - b.Z).Magnitude
end
M.flatDist = flatDist

-- A plain block person, used only if the engine cannot build an avatar (for example with no internet in Studio).
local function fallbackModel(opts)
	local model = Instance.new("Model")
	local hrp = Instance.new("Part")
	hrp.Name = "HumanoidRootPart"
	hrp.Size = Vector3.new(2, 2, 1)
	hrp.Transparency = 1
	hrp.Parent = model
	local function block(name, size, offset, color)
		local p = Instance.new("Part")
		p.Name = name
		p.Size = size
		p.Color = color
		p.CanCollide = false
		p.Massless = true
		p.CFrame = hrp.CFrame * CFrame.new(offset)
		local w = Instance.new("WeldConstraint")
		w.Part0 = hrp
		w.Part1 = p
		w.Parent = p
		p.Parent = model
		return p
	end
	block("Torso", Vector3.new(2, 2, 1), Vector3.new(0, 0, 0), opts.shirt)
	block("Head", Vector3.new(1.4, 1.4, 1.4), Vector3.new(0, 1.7, 0), opts.skin)
	block("Legs", Vector3.new(2, 2, 1), Vector3.new(0, -2, 0), opts.pants)
	local hum = Instance.new("Humanoid")
	hum.HipHeight = 3
	hum.Parent = model
	model.PrimaryPart = hrp
	return model
end

-- opts: name, parent, pos, skin, shirt, pants (Color3), speed, tag (show the name above the head)
function M.make(opts)
	local model
	local ok = pcall(function()
		local desc = Instance.new("HumanoidDescription")
		desc.HeadColor = opts.skin
		desc.TorsoColor = opts.shirt
		desc.LeftArmColor = opts.shirt
		desc.RightArmColor = opts.shirt
		desc.LeftLegColor = opts.pants
		desc.RightLegColor = opts.pants
		model = Players:CreateHumanoidModelFromDescription(desc, Enum.HumanoidRigType.R15)
	end)
	if not ok or not model then
		model = fallbackModel(opts)
	end
	model.Name = opts.name
	local hum = model:FindFirstChildOfClass("Humanoid")
	hum.WalkSpeed = opts.speed or 12
	hum.DisplayName = opts.name
	hum.DisplayDistanceType = opts.tag and Enum.HumanoidDisplayDistanceType.Viewer or Enum.HumanoidDisplayDistanceType.None
	hum.NameDisplayDistance = 25
	hum.HealthDisplayType = Enum.HumanoidHealthDisplayType.AlwaysOff
	model:PivotTo(CFrame.new(opts.pos))
	model.Parent = opts.parent
	local hrp = model:FindFirstChild("HumanoidRootPart")
	if hrp then
		pcall(function()
			hrp:SetNetworkOwner(nil)
		end)
	end
	return model, hum, hrp
end

-- Walks to `target` along a path. `valid()` is checked often; return false from it to abandon the walk.
function M.goTo(hum, hrp, target, valid, maxSeconds)
	local deadline = os.clock() + (maxSeconds or 20)
	local path = PathfindingService:CreatePath({ AgentRadius = 2, AgentHeight = 5, AgentCanJump = true })
	local ok = pcall(function()
		path:ComputeAsync(hrp.Position, target)
	end)
	if not ok or path.Status ~= Enum.PathStatus.Success then
		hum:MoveTo(target)
		local t0 = os.clock()
		while valid() and os.clock() < deadline and os.clock() - t0 < 1.5 and flatDist(hrp.Position, target) > 3 do
			task.wait(0.1)
		end
		return
	end
	for i, wp in ipairs(path:GetWaypoints()) do
		if i > 1 then
			if not valid() or os.clock() > deadline then
				return
			end
			if wp.Action == Enum.PathWaypointAction.Jump then
				hum.Jump = true
			end
			hum:MoveTo(wp.Position)
			local t0 = os.clock()
			while valid() and os.clock() - t0 < 1.5 and flatDist(hrp.Position, wp.Position) > 3 do
				task.wait(0.08)
			end
		end
	end
end

return M

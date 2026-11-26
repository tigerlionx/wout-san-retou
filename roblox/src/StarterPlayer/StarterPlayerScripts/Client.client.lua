-- Wout San Retou (Roblox edition): the player's screen. Draws the HUD and sends stance and flash requests to the server.
-- The server decides speed, Weight and everything that matters; this script only asks.
local Players = game:GetService("Players")
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local UserInputService = game:GetService("UserInputService")
local TweenService = game:GetService("TweenService")

local Config = require(ReplicatedStorage:WaitForChild("WSRConfig"))
local remotes = ReplicatedStorage:WaitForChild("WSRRemotes")
local player = Players.LocalPlayer
local playerGui = player:WaitForChild("PlayerGui")

local INK = Color3.fromRGB(235, 232, 225)
local DIM = Color3.fromRGB(170, 168, 160)
local GOLD = Color3.fromRGB(230, 170, 60)

local gui = Instance.new("ScreenGui")
gui.Name = "WSR_HUD"
gui.ResetOnSpawn = false
gui.IgnoreGuiInset = true
gui.Parent = playerGui

local function label(parent, text, size, pos, textSize, color, align)
	local l = Instance.new("TextLabel")
	l.BackgroundTransparency = 1
	l.Size = size
	l.Position = pos
	l.Text = text
	l.TextSize = textSize
	l.TextColor3 = color or INK
	l.Font = Enum.Font.GothamMedium
	l.TextXAlignment = align or Enum.TextXAlignment.Left
	l.TextWrapped = true
	l.Parent = parent
	return l
end

-- Top left: the objective.
local panel = Instance.new("Frame")
panel.BackgroundColor3 = Color3.fromRGB(10, 10, 14)
panel.BackgroundTransparency = 0.35
panel.Size = UDim2.fromOffset(330, 112)
panel.Position = UDim2.fromOffset(16, 16)
panel.BorderSizePixel = 0
panel.Parent = gui
Instance.new("UICorner", panel).CornerRadius = UDim.new(0, 8)
label(panel, "WOUT SAN RETOU", UDim2.new(1, -20, 0, 22), UDim2.fromOffset(12, 8), 14, GOLD)
local objective = label(panel, "", UDim2.new(1, -20, 0, 44), UDim2.fromOffset(12, 32), 17)
local counts = label(panel, "", UDim2.new(1, -20, 0, 24), UDim2.fromOffset(12, 80), 14, DIM)

-- Top centre: how alert the guards are.
local alertBack = Instance.new("Frame")
alertBack.BackgroundColor3 = Color3.fromRGB(10, 10, 14)
alertBack.BackgroundTransparency = 0.35
alertBack.AnchorPoint = Vector2.new(0.5, 0)
alertBack.Position = UDim2.new(0.5, 0, 0, 20)
alertBack.Size = UDim2.fromOffset(260, 14)
alertBack.BorderSizePixel = 0
alertBack.Parent = gui
Instance.new("UICorner", alertBack).CornerRadius = UDim.new(1, 0)
local alertFill = Instance.new("Frame")
alertFill.BorderSizePixel = 0
alertFill.Size = UDim2.fromScale(0, 1)
alertFill.BackgroundColor3 = Color3.fromRGB(240, 200, 70)
alertFill.Parent = alertBack
Instance.new("UICorner", alertFill).CornerRadius = UDim.new(1, 0)
label(gui, "GUARDS", UDim2.fromOffset(260, 16), UDim2.new(0.5, -130, 0, 38), 11, DIM, Enum.TextXAlignment.Center)

-- Bottom left: stance and Weight.
local status = Instance.new("Frame")
status.BackgroundColor3 = Color3.fromRGB(10, 10, 14)
status.BackgroundTransparency = 0.35
status.AnchorPoint = Vector2.new(0, 1)
status.Position = UDim2.new(0, 16, 1, -16)
status.Size = UDim2.fromOffset(230, 74)
status.BorderSizePixel = 0
status.Parent = gui
Instance.new("UICorner", status).CornerRadius = UDim.new(0, 8)
local stanceLabel = label(status, "", UDim2.new(1, -20, 0, 24), UDim2.fromOffset(12, 8), 16)
local weightLabel = label(status, "", UDim2.new(1, -20, 0, 24), UDim2.fromOffset(12, 32), 16, GOLD)
local flashLabel = label(status, "", UDim2.new(1, -20, 0, 18), UDim2.fromOffset(12, 54), 13, DIM)

-- Bottom centre: key hints.
local hints = label(gui, "Shift run   C sneak   F flash   E hold to use", UDim2.fromOffset(420, 20), UDim2.new(0.5, -210, 1, -34), 13, DIM, Enum.TextXAlignment.Center)
hints.Visible = not UserInputService.TouchEnabled

-- Centre: messages.
local message = label(gui, "", UDim2.fromOffset(560, 60), UDim2.new(0.5, -280, 0.72, 0), 22, INK, Enum.TextXAlignment.Center)
message.TextTransparency = 1
message.TextStrokeTransparency = 1
local messageToken = 0
local function show(text)
	messageToken += 1
	local mine = messageToken
	message.Text = text
	message.TextTransparency = 0
	message.TextStrokeTransparency = 0.6
	task.delay(4.5, function()
		if messageToken == mine then
			TweenService:Create(message, TweenInfo.new(1), { TextTransparency = 1, TextStrokeTransparency = 1 }):Play()
		end
	end)
end

-- Full-screen fade and the ending card.
local fade = Instance.new("Frame")
fade.BackgroundColor3 = Color3.new(0, 0, 0)
fade.BackgroundTransparency = 1
fade.Size = UDim2.fromScale(1, 1)
fade.ZIndex = 10
fade.Parent = gui
local fadeText = label(fade, "", UDim2.fromOffset(600, 120), UDim2.new(0.5, -300, 0.4, 0), 28, INK, Enum.TextXAlignment.Center)
fadeText.ZIndex = 11

-- Touch buttons (phones and tablets).
local function touchButton(text, y, onDown, onUp)
	local b = Instance.new("TextButton")
	b.Text = text
	b.Font = Enum.Font.GothamBold
	b.TextSize = 16
	b.TextColor3 = INK
	b.BackgroundColor3 = Color3.fromRGB(20, 20, 28)
	b.BackgroundTransparency = 0.25
	b.Size = UDim2.fromOffset(84, 56)
	b.AnchorPoint = Vector2.new(1, 1)
	b.Position = UDim2.new(1, -16, 1, y)
	b.Parent = gui
	Instance.new("UICorner", b).CornerRadius = UDim.new(0, 10)
	b.InputBegan:Connect(function(input)
		if input.UserInputType == Enum.UserInputType.Touch or input.UserInputType == Enum.UserInputType.MouseButton1 then
			onDown()
		end
	end)
	b.InputEnded:Connect(function(input)
		if onUp and (input.UserInputType == Enum.UserInputType.Touch or input.UserInputType == Enum.UserInputType.MouseButton1) then
			onUp()
		end
	end)
	return b
end

-- Input: the stance is the sum of "sneak is toggled" and "shift is held".
local sneakToggled = false
local shiftHeld = false
local lastSent = nil
local function sendStance()
	local stance = sneakToggled and "sneak" or (shiftHeld and "sprint" or "walk")
	if stance ~= lastSent then
		lastSent = stance
		remotes.Stance:FireServer(stance)
	end
end

local function flash()
	remotes.Flash:FireServer()
end

UserInputService.InputBegan:Connect(function(input, processed)
	if processed then
		return
	end
	if input.KeyCode == Enum.KeyCode.LeftShift or input.KeyCode == Enum.KeyCode.RightShift then
		shiftHeld = true
		sendStance()
	elseif input.KeyCode == Enum.KeyCode.C then
		sneakToggled = not sneakToggled
		sendStance()
	elseif input.KeyCode == Enum.KeyCode.F then
		flash()
	end
end)
UserInputService.InputEnded:Connect(function(input)
	if input.KeyCode == Enum.KeyCode.LeftShift or input.KeyCode == Enum.KeyCode.RightShift then
		shiftHeld = false
		sendStance()
	end
end)

if UserInputService.TouchEnabled then
	touchButton("RUN", -16, function()
		shiftHeld = true
		sendStance()
	end, function()
		shiftHeld = false
		sendStance()
	end)
	touchButton("SNEAK", -84, function()
		sneakToggled = not sneakToggled
		sendStance()
	end)
	touchButton("FLASH", -152, flash)
end

-- A new character starts upright and walking.
player.CharacterAdded:Connect(function()
	sneakToggled = false
	shiftHeld = false
	lastSent = nil
	sendStance()
end)

-- Updating the HUD from what the server publishes.
local function refresh()
	local gensOff = workspace:GetAttribute("GensOff") or 0
	local gensTotal = workspace:GetAttribute("GensTotal") or 3
	local free = workspace:GetAttribute("KidsFree") or 0
	local safe = workspace:GetAttribute("KidsSafe") or 0
	local total = workspace:GetAttribute("KidsTotal") or #Config.Kids
	if gensOff < gensTotal then
		objective.Text = "Cut the power at every generator."
	elseif free < total then
		objective.Text = "Reach the cellblock and call the Lost by name."
	else
		objective.Text = "Lead them to the road out."
	end
	counts.Text = ("Power cut %d/%d   Reached %d/%d   Safe %d/%d"):format(gensOff, gensTotal, free, total, safe, total)
	local w = player:GetAttribute("Weight") or 0
	weightLabel.Text = "Weight: " .. w
	local stance = player:GetAttribute("Stance") or "walk"
	stanceLabel.Text = "Stance: " .. stance
	local alert = workspace:GetAttribute("Alert") or 0
	alertFill.Size = UDim2.fromScale(alert, 1)
	alertFill.BackgroundColor3 = alert >= 1 and Color3.fromRGB(220, 70, 60) or Color3.fromRGB(240, 200, 70)
end
workspace.AttributeChanged:Connect(refresh)
player.AttributeChanged:Connect(refresh)
refresh()

-- Flash cooldown shown next to the stance.
local readyAt = 0
task.spawn(function()
	while true do
		local left = readyAt - os.clock()
		flashLabel.Text = left > 0 and ("Flash ready in %.0fs"):format(left) or "Flash ready"
		task.wait(0.25)
	end
end)

remotes.Notify.OnClientEvent:Connect(show)
remotes.Flash.OnClientEvent:Connect(function(hit, cooldown)
	readyAt = os.clock() + (cooldown or 0)
	if hit and hit > 0 then
		fade.BackgroundColor3 = Color3.new(1, 1, 1)
		fade.BackgroundTransparency = 0.2
		TweenService:Create(fade, TweenInfo.new(0.5), { BackgroundTransparency = 1 }):Play()
	end
end)
remotes.Detained.OnClientEvent:Connect(function(text)
	fade.BackgroundColor3 = Color3.new(0, 0, 0)
	fadeText.Text = text
	TweenService:Create(fade, TweenInfo.new(0.8), { BackgroundTransparency = 0 }):Play()
	task.delay(2.4, function()
		fadeText.Text = ""
		TweenService:Create(fade, TweenInfo.new(0.8), { BackgroundTransparency = 1 }):Play()
	end)
end)
remotes.Ending.OnClientEvent:Connect(function(text, safe)
	fade.BackgroundColor3 = Color3.new(0, 0, 0)
	fadeText.Text = ("%d of %d reached the road.\n\n%s"):format(safe, #Config.Kids, text)
	TweenService:Create(fade, TweenInfo.new(2), { BackgroundTransparency = 0.15 }):Play()
	task.delay(13, function()
		fadeText.Text = ""
		TweenService:Create(fade, TweenInfo.new(1), { BackgroundTransparency = 1 }):Play()
	end)
end)

-- First screen: what to do.
show("Cut the power. Reach the cellblock. Call them by name. Lead them to the road.")

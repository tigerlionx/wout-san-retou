-- Shared settings for Wout San Retou (Roblox edition). Every number you may want to tune is here.
local Config = {}

Config.Title = "Wout San Retou: The Road of No Return"

-- Studs per second. The server sets these; the client only asks for a stance.
Config.Speed = { walk = 14, sprint = 22, sneak = 7 }

-- Weight: the cost of using force. It slows you a little and calls an Echo that follows you.
Config.Weight = {
	slowPerPoint = 0.03,
	minMultiplier = 0.75,
	maxEchoes = 6,
	echoSpeed = 9,
	echoTouch = 4,
	echoSlowSeconds = 3,
	echoSlowMultiplier = 0.6,
	echoRespawnSeconds = 10,
}

Config.Guard = {
	patrolSpeed = 8,
	investigateSpeed = 12,
	chaseSpeed = 17,
	sightRange = 45, -- studs, in the dark with all lights on
	sightHalfAngle = 55, -- degrees either side of where the guard faces
	closeSense = 7, -- inside this a guard notices you from any direction
	catchDistance = 4,
	loseSeconds = 4, -- chase ends this long after the guard last saw you
}

-- How much each stance changes how far a guard can see you.
Config.StanceSight = { walk = 1, sprint = 1.3, sneak = 0.5 }
-- How far a guard can hear you, but only while you are moving.
Config.Hearing = { walk = 12, sprint = 36, sneak = 3 }

-- Cutting a generator makes the yard darker, so guards see less far: each one removes this share of sight range.
Config.DarknessPerGenerator = 0.15

-- The blinding flash. It does no damage, but using it on someone adds Weight.
Config.Flash = { range = 26, halfAngle = 35, stunSeconds = 6, cooldown = 8, missCooldown = 2, weightCost = 1 }

Config.Hold = { generator = 3, call = 2 } -- seconds to hold E

-- The Lost: kids caught in the gang. They are reached by name, never fought.
Config.Kids = {
	{ name = "Wilson", age = 14, line = "Wilson. Your mother is still waiting at the school gate." },
	{ name = "Marie", age = 13, line = "Marie. Your little brother asks about you every day." },
	{ name = "Jephte", age = 16, line = "Jephte. Your coach kept your place on the team." },
}

return Config

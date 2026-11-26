-- A counter that identifies the current round. Every background loop remembers the id it started with and stops when it changes.
local Round = { id = 0 }
return Round

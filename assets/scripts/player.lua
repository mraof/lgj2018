function init()
    object.rotate_cooldown = 0
    object.gravity_cooldown = 0
end

function update()
    local horizontal = ((controls.left and -1 or 0) + (controls.right and 1 or 0)) * delta * 180
    if (horizontal > 0) then
        object:flip(false)
    elseif horizontal < 0 then
        object:flip(true)
    end
    local vertical = delta * gravity

    object:move((horizontal), vertical)

    if (object.rotate_cooldown <= 0)
    then
        if (controls.a)
        then
            object:rotate(object.rotation + 1)
            object.rotate_cooldown = 0.1
        end
    else
        object.rotate_cooldown = object.rotate_cooldown - delta
    end

    if (object.gravity_cooldown <= 0)
    then
        if (controls.b)
        then
            gravity = gravity + 25
            object.gravity_cooldown = 0.5
        end
    else
        object.gravity_cooldown = object.gravity_cooldown - delta
    end
end

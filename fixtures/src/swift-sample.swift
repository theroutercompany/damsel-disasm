@inline(never)
public func publicAdd(_ value: Int) -> Int {
    value + 7
}

@inline(never)
public func chooseValue(_ value: Int) -> Int {
    switch value & 3 {
    case 0:
        return publicAdd(value)
    case 1:
        return value * 3
    case 2:
        return value - 11
    default:
        return value ^ 0x55
    }
}

@inline(never)
func internalMessage(_ seed: Int) -> String {
    "swift-\(seed)"
}

let seed = CommandLine.arguments.count
print(publicAdd(seed))
print(chooseValue(seed))
print(internalMessage(seed))

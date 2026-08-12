//
//  Item.swift
//  Airkeys
//
//  Created by shelchin on 2026/8/12.
//

import Foundation
import SwiftData

@Model
final class Item {
    var timestamp: Date
    
    init(timestamp: Date) {
        self.timestamp = timestamp
    }
}

package net.eternaltwin.twinoid

import kotlinx.serialization.KSerializer
import kotlinx.serialization.Serializable
import kotlinx.serialization.descriptors.PrimitiveKind
import kotlinx.serialization.descriptors.PrimitiveSerialDescriptor
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder

@Serializable(with = TwinoidUserDisplayName.Serializer::class)
data class TwinoidUserDisplayName(
  val value: String,
) {
  override fun toString(): String = "TwinoidUserDisplayName(${this.value})"

  object Serializer : KSerializer<TwinoidUserDisplayName> {
    override val descriptor: SerialDescriptor =
      PrimitiveSerialDescriptor("net.eternaltwin.user.UserId", PrimitiveKind.STRING)

    override fun serialize(encoder: Encoder, value: TwinoidUserDisplayName) =
      encoder.encodeString(value.value)

    override fun deserialize(decoder: Decoder): TwinoidUserDisplayName =
      TwinoidUserDisplayName(decoder.decodeString())
  }
}

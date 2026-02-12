export interface Artwork {
  id: string;
  title: string;
  artist: string;
  date: string;
  medium: string;
  source: string;
  image_base64: string;
}

export interface Settings {
  hotkey: string;
}

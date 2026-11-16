import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Flip the Table",
  description: "Retro physics-driven table flipping mayhem.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
